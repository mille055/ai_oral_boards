use lambda_runtime::{run, service_fn, Error, LambdaEvent};
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

#[derive(Deserialize)]
struct Request {
    #[serde(rename = "httpMethod")]
    http_method: String,
    path: String,
    #[serde(rename = "pathParameters")]
    #[allow(dead_code)]
    path_parameters: Option<HashMap<String, String>>,
    #[serde(rename = "queryStringParameters")]
    #[allow(dead_code)]
    query_parameters: Option<HashMap<String, String>>,
    #[serde(rename = "isBase64Encoded")]
    is_base64_encoded: Option<bool>,
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
async fn serve_frontend(path: &str) -> Result<Response, Error> {
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
            info!("Frontend file not found: {}", path);
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

            info!("Serving frontend file: {}", file_path);

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
        Err(_) => {
            info!("Frontend file read error: {}", file_path);
            // File not found
            Ok(Response {
                status_code: 404,
                headers: HashMap::from([
                    ("Content-Type".to_string(), "text/plain".to_string()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                ]),
                is_base64_encoded: false,
                body: "File Not Found".to_string(),
            })
        }
    }
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    info!("Lambda function invoked");
    
    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);
    
    let request = event.payload;
    let context = event.context;

    info!(
        "Processing request: {} {} (request_id: {})",
        request.http_method, request.path, context.request_id
    );

    // First, handle frontend file serving for non-API routes
    if !request.path.starts_with("/api") {
        match serve_frontend(&request.path).await {
            Ok(response) => return Ok(response),
            Err(_) => {} // Continue to API route handling
        }
    }

    if request.http_method == "OPTIONS" {
        return Ok(Response::new(200, "OK")?);
    }

    let result = match (request.http_method.as_str(), request.path.as_str()) {
        ("GET", "/api/cases") => {
            info!("Handling GET /api/cases - Listing all cases");
            let cases = db::list_cases(&dynamodb_client).await?;
            info!("Found {} cases", cases.len());
            Ok::<Response, Error>(Response::new(200, ApiResponse::success(cases)).map_err(anyhow::Error::from)?)
        },
        
        ("GET", p) if p.starts_with("/api/cases/") && p.split('/').count() == 4 => {
            let parts: Vec<&str> = p.split('/').collect();
            let case_id = parts[3];

            let case = db::get_case(&dynamodb_client, case_id).await?;
            match case {
                Some(case) => Ok(Response::new(200, ApiResponse::success(case))?),
                None => Ok(Response::new(404, ErrorResponse::not_found("Case not found"))?),
            }
        },

        ("GET", p) if p.starts_with("/api/cases/") && p.contains("/images/") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() < 6 {
                return Ok(Response::new(400, ErrorResponse::bad_request("Invalid image path"))?);
            }
            
            let case_id = parts[3];
            let image_id = parts[5];
            
            let case = match db::get_case(&dynamodb_client, case_id).await? {
                Some(case) => case,
                None => return Ok(Response::new(404, ErrorResponse::not_found("Case not found"))?),
            };
            
            if !case.image_ids.contains(&image_id.to_string()) {
                return Ok(Response::new(404, ErrorResponse::not_found("Image not found in case"))?);
            }
            
            let s3_key = format!("dicom/{}/{}.dcm", case_id, image_id);
            let image_data = s3::download_file(&s3_client, &s3_key).await?;
            
            Ok(Response::new(200, "OK")?
                .with_content_type("application/dicom")
                .into_binary(image_data))
        },

        ("POST", "/api/cases") => {
            if let Some(body) = request.body {
                let decoded_body = if request.is_base64_encoded.unwrap_or(false) {
                    String::from_utf8(BASE64.decode(body)?)?
                } else {
                    body
                };
                
                let case_upload: models::CaseUpload = serde_json::from_str(&decoded_body)
                    .context("Failed to parse case upload data")?;
                
                let dicom_data = BASE64.decode(&case_upload.dicom_file)?;
                let metadata = dicom::extract_metadata(&dicom_data)
                    .context("Failed to extract DICOM metadata")?;
                
                let case_id = uuid::Uuid::new_v4().to_string();
                
                let s3_key = format!("dicom/{}/{}.dcm", case_id, metadata.sop_instance_uid);
                s3::upload_file(&s3_client, &s3_key, dicom_data).await?;
                
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
                
                db::save_case(&dynamodb_client, &case).await?;
                Ok(Response::new(201, ApiResponse::success(case))?)
            } else {
                Ok(Response::new(400, ErrorResponse::bad_request("Missing request body"))?)
            }
        },

        _ => Ok(Response::new(404, ErrorResponse::not_found("Route not found"))?),
    };

    match result {
        Ok(response) => Ok::<Response, lambda_runtime::Error>(response),  
        Err(err) => {
            error!("Error processing request: {:?}", err);
            let error_response = Response::new(500, ErrorResponse::server_error(format!("{:?}", err)))?;
            Ok::<Response, lambda_runtime::Error>(error_response)  
        }
    }    
}

#[tokio::main]
async fn main() -> Result<(), Error> {
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