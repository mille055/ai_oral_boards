use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use anyhow::{Context, Result};
use tracing::{info, error};

mod db;
mod s3;
mod dicom;
mod models;

use models::{Case, CaseMetadata, ApiResponse, ErrorResponse};

#[derive(Deserialize)]
struct Request {
    #[serde(rename = "httpMethod")]
    http_method: String,
    path: String,
    #[serde(rename = "pathParameters")]
    path_parameters: Option<HashMap<String, String>>,
    #[serde(rename = "queryStringParameters")]
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

    fn as_binary(mut self, data: Vec<u8>) -> Self {
        self.is_base64_encoded = true;
        self.body = base64::encode(data);
        self
    }
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    info!("Lambda function invoked");
    
    // Load AWS config
    info!("Loading AWS configuration");
    let config = aws_config::load_from_env().await;
    
    // Initialize DynamoDB and S3 clients
    info!("Initializing DynamoDB client");
    let dynamodb_client = DynamoDbClient::new(&config);
    
    info!("Initializing S3 client");
    let s3_client = S3Client::new(&config);
    
    let request = event.payload;
    let context = event.context;

    info!(
        "Processing request: {} {} (request_id: {})",
        request.http_method, request.path, context.request_id
    );

    // Handle CORS preflight requests
    if request.http_method == "OPTIONS" {
        return Ok(Response::new(200, "OK")?);
    }

    let result = match (request.http_method.as_str(), request.path.as_str()) {
        // List all cases
        ("GET", "/api/cases") => {
            info!("Handling GET /api/cases - Listing all cases");
            let cases = db::list_cases(&dynamodb_client).await?;
            info!("Found {} cases", cases.len());
            Ok(Response::new(200, ApiResponse::success(cases))?)
        },
        
        // Get a specific case
        ("GET", p) if p.starts_with("/api/cases/") && p.split('/').count() == 4 => {
            info!("Handling GET case details: {}", p);
            let parts: Vec<&str> = p.split('/').collect();
            let case_id = parts[3];
            info!("Getting case with ID: {}", case_id);
            
            let case = db::get_case(&dynamodb_client, case_id).await?;
            match case {
                Some(case) => {
                    info!("Found case: {} - {}", case_id, case.title);
                    Ok(Response::new(200, ApiResponse::success(case))?)
                },
                None => {
                    info!("Case not found: {}", case_id);
                    Ok(Response::new(404, ErrorResponse::not_found("Case not found"))?)
                }
            }
        },
        
        // Get a specific image from a case
        ("GET", p) if p.starts_with("/api/cases/") && p.contains("/images/") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() < 6 {
                return Ok(Response::new(400, ErrorResponse::bad_request("Invalid image path"))?);
            }
            
            let case_id = parts[3];
            let image_id = parts[5];
            
            // Get the case to verify the image belongs to it
            let case = match db::get_case(&dynamodb_client, case_id).await? {
                Some(case) => case,
                None => return Ok(Response::new(404, ErrorResponse::not_found("Case not found"))?)
            };
            
            // Check if the image belongs to this case
            if !case.image_ids.contains(&image_id.to_string()) {
                return Ok(Response::new(404, ErrorResponse::not_found("Image not found in case"))?);
            }
            
            // Get the image from S3
            let s3_key = format!("dicom/{}/{}.dcm", case_id, image_id);
            let image_data = s3::download_file(&s3_client, &s3_key).await?;
            
            // Return the image data
            // In a real implementation, you might want to convert DICOM to PNG or JPG here
            Ok(Response::new(200, "OK")?
                .with_content_type("application/dicom")
                .as_binary(image_data))
        },
        
        // Upload a new case
        ("POST", "/api/cases") => {
            info!("Handling POST /api/cases - Creating new case");
            if let Some(body) = request.body {
                // If the body is base64 encoded, decode it
                let decoded_body = if request.is_base64_encoded.unwrap_or(false) {
                    info!("Decoding base64 request body");
                    String::from_utf8(base64::decode(body)?)?
                } else {
                    body
                };
                
                // Parse the case data from the request body
                info!("Parsing case upload data");
                let case_upload: models::CaseUpload = serde_json::from_str(&decoded_body)
                    .context("Failed to parse case upload data")?;
                
                info!("Processing DICOM file data");
                // Process DICOM file and extract metadata
                let dicom_data = base64::decode(&case_upload.dicom_file)?;
                info!("DICOM file size: {} bytes", dicom_data.len());
                
                info!("Extracting DICOM metadata");
                let metadata = dicom::extract_metadata(&dicom_data)
                    .context("Failed to extract DICOM metadata")?;
                
                // Generate a unique case ID
                let case_id = uuid::Uuid::new_v4().to_string();
                info!("Generated case ID: {}", case_id);
                
                // Upload DICOM file to S3
                let s3_key = format!("dicom/{}/{}.dcm", case_id, metadata.sop_instance_uid);
                info!("Uploading DICOM file to S3: {}", s3_key);
                s3::upload_file(&s3_client, &s3_key, dicom_data).await?;
                
                // Create a case record
                info!("Creating case record: {} - {}", case_id, case_upload.title);
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
                
                // Log important DICOM metadata
                info!("DICOM metadata: SOP Instance UID: {}, Modality: {}", 
                      metadata.sop_instance_uid, metadata.modality);
                
                // Save case metadata to DynamoDB
                info!("Saving case metadata to DynamoDB");
                db::save_case(&dynamodb_client, &case).await?;
                
                info!("Case created successfully: {}", case_id);
                Ok(Response::new(201, ApiResponse::success(case))?)
            } else {
                Ok(Response::new(400, ErrorResponse::bad_request("Missing request body"))?)
            }
        },
        
        // Upload multiple DICOM files as a single case/exam
        ("POST", "/api/cases/upload-exam") => {
            // In a real implementation, you would handle multipart form data here
            // For now, we'll return a not implemented response
            Ok(Response::new(501, ErrorResponse::not_implemented("Multi-file upload endpoint is under development"))?)
        },
        
        // DICOM web compatible endpoints for OHIF viewer
        ("GET", p) if p.starts_with("/dicomweb/studies") => {
            // Implement DICOM web standards for OHIF viewer
            // This would involve parsing the path and query parameters to determine what data to return
            // For now, we'll return a not implemented response
            Ok(Response::new(501, ErrorResponse::not_implemented("DICOM web endpoint is under development"))?)
        },
        
        // Default - route not found
        _ => {
            Ok(Response::new(404, ErrorResponse::not_found("Route not found"))?)
        }
    };

    match result {
        Ok(response) => Ok(response),
        Err(err) => {
            error!("Error processing request: {:?}", err);
            Ok(Response::new(500, ErrorResponse::server_error(format!("{:?}", err)))?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for CloudWatch logs
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Radiology Teaching Files Rust Microservice");
    
    // Ensure required AWS resources exist
    info!("Loading AWS configuration");
    let config = aws_config::load_from_env().await;
    
    // Initialize clients and create resources if they don't exist
    // In a real production environment, this would be handled by IaC
    // but we're explicitly including it to demonstrate database connectivity
    info!("Initializing DynamoDB client");
    let dynamodb_client = DynamoDbClient::new(&config);
    
    info!("Initializing S3 client");
    let s3_client = S3Client::new(&config);
    
    info!("Ensuring DynamoDB table exists");
    if let Err(err) = db::ensure_table_exists(&dynamodb_client).await {
        error!("Failed to ensure DynamoDB table exists: {:?}", err);
        // Continue anyway - in production, the table might be created by Terraform
        info!("Will attempt to continue with existing table or create one on demand");
    } else {
        info!("DynamoDB table confirmed and ready");
    }
    
    info!("Ensuring S3 bucket exists");
    if let Err(err) = s3::ensure_bucket_exists(&s3_client).await {
        error!("Failed to ensure S3 bucket exists: {:?}", err);
        // Continue anyway - in production, the bucket might be created by Terraform
        info!("Will attempt to continue with existing bucket or create one on demand");
    } else {
        info!("S3 bucket confirmed and ready");
    }
    
    info!("Rust microservice initialization complete, ready to process requests");
    
    // Start the Lambda runtime
    info!("Starting Lambda runtime");
    run(service_fn(function_handler)).await
}