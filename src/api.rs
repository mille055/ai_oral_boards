use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use anyhow::Result;
use crate::models::ErrorResponse;

// Request handling
pub mod request {
    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Request {
        #[serde(rename = "httpMethod", default)]
        pub http_method: Option<String>,
        
        #[serde(rename = "path", default)]
        pub path: Option<String>,
        
        #[serde(rename = "rawPath", default)]
        pub raw_path: Option<String>,
        
        #[serde(rename = "requestContext", default)]
        pub request_context: Option<RequestContext>,
        
        #[serde(default)]
        pub body: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct RequestContext {
        #[serde(rename = "http", default)]
        pub http: Option<HttpContext>,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct HttpContext {
        #[serde(rename = "method", default)]
        pub method: Option<String>,
        
        #[serde(rename = "path", default)]
        pub path: Option<String>,
    }

    // Extract method and path from the Lambda request
    pub fn extract_method_and_path(request: &Request) -> (String, String) {
        // Extract method
        let http_method = request.http_method
            .clone()
            .or_else(|| request.request_context.as_ref()
                .and_then(|ctx| ctx.http.as_ref()
                    .and_then(|http| http.method.clone())))
            .unwrap_or_else(|| "UNKNOWN".to_string());
        
        // Extract path
        let path = request.path
            .clone()
            .or_else(|| request.raw_path.clone())
            .or_else(|| request.request_context.as_ref()
                .and_then(|ctx| ctx.http.as_ref()
                    .and_then(|http| http.path.clone())))
            .unwrap_or_else(|| "/".to_string());
        
        (http_method, path)
    }
}

// Response handling
pub mod response {
    use super::*;
    use lambda_runtime::Error as LambdaError;

    #[derive(Serialize)]
    pub struct Response {
        #[serde(rename = "statusCode")]
        pub status_code: u16,
        pub headers: HashMap<String, String>,
        #[serde(rename = "isBase64Encoded")]
        pub is_base64_encoded: bool,
        pub body: String,
    }

    // Create CORS headers
    pub fn create_cors_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Methods".to_string(), 
                      "GET, POST, PUT, DELETE, OPTIONS".to_string());
        headers.insert("Access-Control-Allow-Headers".to_string(), 
                      "Content-Type, Authorization, X-Requested-With".to_string());
        
        headers
    }

    // Common responses
    pub fn options_response() -> Response {
        Response {
            status_code: 200,
            headers: create_cors_headers(),
            is_base64_encoded: false,
            body: "".to_string(),
        }
    }

    pub fn not_found(message: &str) -> Result<Response, LambdaError> {
        Response::new(404, ErrorResponse::not_found(message))
    }

    pub fn bad_request(message: &str) -> Result<Response, LambdaError> {
        Response::new(400, ErrorResponse::bad_request(message))
    }

    pub fn server_error(message: &str) -> Result<Response, LambdaError> {
        Response::new(500, ErrorResponse::server_error(message.to_string()))
    }
    
    impl Response {
        pub fn new(status_code: u16, body: impl Serialize) -> Result<Self, LambdaError> {
            let headers = create_cors_headers();
            
            Ok(Self {
                status_code,
                headers,
                is_base64_encoded: false,
                body: serde_json::to_string(&body)?,
            })
        }
        
        pub fn with_content_type(mut self, content_type: &str) -> Self {
            self.headers.insert("Content-Type".to_string(), content_type.to_string());
            self
        }
        
        pub fn into_binary(mut self, data: Vec<u8>) -> Self {
            self.is_base64_encoded = true;
            self.body = BASE64.encode(data);
            self
        }
    }
}