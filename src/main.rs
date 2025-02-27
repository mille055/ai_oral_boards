use lambda_runtime::{run, service_fn, LambdaEvent, Error as LambdaError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use aws_sdk_dynamodb::Client as DynamoDbClient;
// use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use aws_sdk_s3::Client as S3Client;
use anyhow::Result;
use tracing::error;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::env;

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
    
    // #[serde(rename = "pathParameters", default)]
    // path_parameters: Option<HashMap<String, String>>,
    
    // #[serde(rename = "queryStringParameters", default)]
    // query_parameters: Option<HashMap<String, String>>,
    
    // #[serde(rename = "isBase64Encoded", default)]
    // is_base64_encoded: Option<bool>,
    
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
    #[allow(dead_code)]
    fn with_content_type(mut self, content_type: &str) -> Self {
        self.headers.insert("Content-Type".to_string(), content_type.to_string());
        self
    }
    #[allow(dead_code)]
    fn into_binary(mut self, data: Vec<u8>) -> Self {
        self.is_base64_encoded = true;
        self.body = BASE64.encode(data);
        self
    }
}

/// Serve static frontend files from S3
async fn serve_frontend(s3_client: &S3Client, path: &str) -> Result<Response, LambdaError> {
    let bucket_name = env::var("S3_BUCKET").unwrap_or_else(|_| "radiology-teaching-files".to_string());
    let key = format!("frontend{}", path);

    match s3_client.get_object()
        .bucket(bucket_name)
        .key(&key)
        .send()
        .await {
        Ok(output) => {
            let body = output.body.collect().await?.into_bytes();
            let content_type = match path.split('.').last() {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                _ => "text/plain; charset=utf-8",
            };

            Ok(Response {
                status_code: 200,
                headers: HashMap::from([
                    ("Content-Type".to_string(), content_type.to_string()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                ]),
                is_base64_encoded: true,
                body: BASE64.encode(body),
            })
        }
        Err(e) => {
            println!("Frontend file read error: {:?} - {}", key, e);
            Ok(Response::new(404, "File Not Found")?)
        }
    }
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, LambdaError> {
    println!("FULL EVENT DUMP: {:?}", event);
    
    let request = event.payload;
    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);

    let http_method = request.http_method.as_deref().unwrap_or("UNKNOWN");
    let path = request.path.as_deref().unwrap_or("/");

    println!("PROCESSED REQUEST: method={}, path={}", http_method, path);

    if !path.starts_with("/api") {
        return serve_frontend(&s3_client, path).await;
    }

    let result = match (http_method, path) {
        ("GET", "/api/cases") => {
            let cases = db::list_cases(&dynamodb_client).await?;
            Ok(Response::new(200, ApiResponse::success(cases))?)
        },
        ("POST", "/api/cases") => {
            if let Some(body) = request.body {
                let case_upload: models::CaseUpload = serde_json::from_str(&body)?;
                let dicom_data = BASE64.decode(&case_upload.dicom_file)?;
                let metadata = dicom::extract_metadata(&dicom_data)?;

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
        _ => {
            Ok(Response::new(404, ErrorResponse::not_found("Route not found"))?)
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
