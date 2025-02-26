use lambda_runtime::{run, service_fn, LambdaEvent, Error as LambdaError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use anyhow::{Context, Result};
use tracing::{info, error};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::path::Path;

mod db;
mod s3;
mod dicom;
mod models;

use models::{Case, ApiResponse, ErrorResponse};

#[derive(Deserialize, Debug)]
struct Request {
    #[serde(rename = "httpMethod", default)]
    http_method: Option<String>,
    
    #[serde(rename = "path", default)]
    path: Option<String>,
    
    #[serde(rename = "pathParameters", default)]
    path_parameters: Option<HashMap<String, String>>,
    
    #[serde(rename = "queryStringParameters", default)]
    query_parameters: Option<HashMap<String, String>>,
    
    #[serde(rename = "isBase64Encoded", default)]
    is_base64_encoded: Option<bool>,
    
    #[serde(default)]
    body: Option<String>,
}

#[derive(Serialize)]
struct Response {
    #[serde(rename = "statusCode")]
    status_code: u16,
    headers: HashMap<String, String>,
    #[serde(rename = "isBase64Encoded")]
    is_base64_encoded: bool,
    body: String,
}

impl Response {
    fn new(status_code: u16, body: impl Serialize) -> Result<Self> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, OPTIONS".to_string());
        headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type".to_string());

        Ok(Self {
            status_code,
            headers,
            is_base64_encoded: false,
            body: serde_json::to_string(&body)?,
        })
    }

    fn with_content_type(mut self, content_type: &str) -> Self {
        self.headers.insert("Content-Type".to_string(), content_type.to_string());
        self
    }

    fn into_binary(mut self, data: Vec<u8>) -> Self {
        self.is_base64_encoded = true;
        self.body = BASE64.encode(data);
        self
    }
}

/// Serve static frontend files
async fn serve_frontend(path: &str) -> Result<Response, LambdaError> {
    println!("Attempting to serve frontend file for path: {}", path);
    
    // Current working directory logging
    let current_dir = std::env::current_dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|_| "Could not determine current directory".to_string());
    println!("Current working directory: {}", current_dir);

    // Map routes to file paths
    let file_path = match path {
        "/" | "/index.html" => "frontend/index.html",
        "/case.html" => "frontend/case.html",
        "/upload.html" => "frontend/upload.html",
        "/js/main.js" => "frontend/js/main.js",
        "/js/case.js" => "frontend/js/case.js",
        "/js/upload.js" => "frontend/js/upload.js",
        "/styles.css" => "frontend/styles.css",
        _ => {
            println!("Frontend file not found: {}", path);
            return Ok(Response {
                status_code: 404,
                headers: HashMap::from([
                    ("Content-Type".to_string(), "text/plain".to_string()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                ]),
                is_base64_encoded: false,
                body: "Not Found".to_string(),
            });
        }
    };

    // Read the file contents
    match std::fs::read_to_string(file_path) {
        Ok(content) => {
            // Determine content type based on file extension
            let content_type = match Path::new(file_path).extension()
                .and_then(|ext| ext.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                _ => "text/plain; charset=utf-8",
            };

            println!("Serving frontend file: {}", file_path);

            // Return the file contents
            Ok(Response {
                status_code: 200,
                headers: HashMap::from([
                    ("Content-Type".to_string(), content_type.to_string()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                ]),
                is_base64_encoded: false,
                body: content,
            })
        }
        Err(e) => {
            println!("Frontend file read error: {} - {}", file_path, e);
            // File not found
            Ok(Response {
                status_code: 404,
                headers: HashMap::from([
                    ("Content-Type".to_string(), "text/plain".to_string()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                ]),
                is_base64_encoded: false,
                body: format!("File Not Found: {}", e),
            })
        }
    }
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, LambdaError> {
    // EXTREMELY VERBOSE LOGGING
    println!("FULL EVENT DUMP: {:?}", event);
    
    // Log raw payload details with more context
    let request = event.payload;
    
    // Log payload details
    println!("DIAGNOSTIC INFO:");
    println!("HTTP METHOD: {:?}", request.http_method);
    println!("PATH: {:?}", request.path);
    println!("BODY: {:?}", request.body);
    println!("PATH PARAMETERS: {:?}", request.path_parameters);
    println!("QUERY PARAMETERS: {:?}", request.query_parameters);

    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);
    
    // Extract method and path with fallback
    let http_method = request.http_method.as_deref().unwrap_or("UNKNOWN");
    let path = request.path.as_deref().unwrap_or("/");

    println!("PROCESSED REQUEST: method={}, path={}", http_method, path);

    // Attempt to handle frontend files first
    if !path.starts_with("/api") {
        match serve_frontend(path).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                println!("Frontend serving error: {:?}", e);
            }
        }
    }

    // Main routing logic with extensive logging
    let result = match (http_method, path) {
        ("GET", "/api/cases") => {
            println!("Matched GET /api/cases route");
            let cases = db::list_cases(&dynamodb_client).await
                .map_err(|e| LambdaError::from(e))?;
            println!("Found {} cases", cases.len());
            Ok(Response::new(200, ApiResponse::success(cases))
                .map_err(|e| LambdaError::from(e))?)
        },
        
        ("GET", p) if p.starts_with("/api/cases/") && p.split('/').count() == 4 => {
            println!("Matched GET single case route");
            let parts: Vec<&str> = p.split('/').collect();
            let case_id = parts[3];
            println!("Fetching case with ID: {}", case_id);
            
            let case = db::get_case(&dynamodb_client, case_id).await
                .map_err(|e| LambdaError::from(e))?;
            match case {
                Some(case) => Ok(Response::new(200, ApiResponse::success(case))
                    .map_err(|e| LambdaError::from(e))?),
                None => Ok(Response::new(404, ErrorResponse::not_found("Case not found"))
                    .map_err(|e| LambdaError::from(e))?),
            }
        },

        ("GET", p) if p.starts_with("/api/cases/") && p.contains("/images/") => {
            println!("Matched GET case images route");
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 6 {
                let case_id = parts[3];
                let image_id = parts[5];
                
                println!("Fetching image: case_id={}, image_id={}", case_id, image_id);
                
                let case = match db::get_case(&dynamodb_client, case_id).await
                    .map_err(|e| LambdaError::from(e))? {
                    Some(case) => case,
                    None => return Ok(Response::new(404, ErrorResponse::not_found("Case not found"))
                        .map_err(|e| LambdaError::from(e))?),
                };
                
                if !case.image_ids.contains(&image_id.to_string()) {
                    return Ok(Response::new(404, ErrorResponse::not_found("Image not found in case"))
                        .map_err(|e| LambdaError::from(e))?);
                }
                
                let s3_key = format!("dicom/{}/{}.dcm", case_id, image_id);
                let image_data = s3::download_file(&s3_client, &s3_key).await
                    .map_err(|e| LambdaError::from(e))?;
                
                Ok(Response::new(200, "OK")
                    .map_err(|e| LambdaError::from(e))?
                    .with_content_type("application/dicom")
                    .into_binary(image_data))
            } else {
                println!("Invalid image route: {}", p);
                Ok(Response::new(400, ErrorResponse::bad_request("Invalid image path"))
                    .map_err(|e| LambdaError::from(e))?)
            }
        },

        ("POST", "/api/cases") => {
            println!("Matched POST /api/cases route");
            if let Some(body) = request.body {
                println!("Received body: {}", body);
                
                let decoded_body = if request.is_base64_encoded.unwrap_or(false) {
                    String::from_utf8(BASE64.decode(body)
                        .map_err(|e| LambdaError::from(e))?)
                        .map_err(|e| LambdaError::from(e))?
                } else {
                    body
                };
                
                println!("Decoded body: {}", decoded_body);
                
                let case_upload: models::CaseUpload = serde_json::from_str(&decoded_body)
                    .map_err(|e| LambdaError::from(e))?;
                
                let dicom_data = BASE64.decode(&case_upload.dicom_file)
                    .map_err(|e| LambdaError::from(e))?;
                let metadata = dicom::extract_metadata(&dicom_data)
                    .map_err(|e| LambdaError::from(e))?;
                
                let case_id = uuid::Uuid::new_v4().to_string();
                
                let s3_key = format!("dicom/{}/{}.dcm", case_id, metadata.sop_instance_uid);
                s3::upload_file(&s3_client, &s3_key, dicom_data).await
                    .map_err(|e| LambdaError::from(e))?;
                
                let case = Case {
                    case_id: case_id.clone(),
                    title: case_upload.title,
                    description: case_upload.description,
                    modality: metadata.modality.clone(),
                    anatomy: case_upload.anatomy,
                    diagnosis: case_upload.diagnosis,
                    findings: case_upload.findings,
                    tags: case_upload.tags,
                    image_ids: vec![metadata.sop_instance_uid.clone()],
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                
                db::save_case(&dynamodb_client, &case).await
                    .map_err(|e| LambdaError::from(e))?;
                Ok(Response::new(201, ApiResponse::success(case))
                    .map_err(|e| LambdaError::from(e))?)
            } else {
                println!("Missing request body");
                Ok(Response::new(400, ErrorResponse::bad_request("Missing request body"))
                    .map_err(|e| LambdaError::from(e))?)
            }
        },

        _ => {
            println!("UNMATCHED ROUTE: method={}, path={}", http_method, path);
            Ok(Response::new(404, ErrorResponse::not_found(
                &format!("Route not found: {} {}", http_method, path)))
                .map_err(|e| LambdaError::from(e))?)
        }
    };

    result
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);

    if let Err(err) = db::ensure_table_exists(&dynamodb_client).await {
        error!("Failed to ensure DynamoDB table exists: {:?}", err);
    }

    if let Err(err) = s3::ensure_bucket_exists(&s3_client).await {
        error!("Failed to ensure S3 bucket exists: {:?}", err);
    }

    run(service_fn(function_handler)).await
}