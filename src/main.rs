use lambda_runtime::{run, service_fn, LambdaEvent, Error as LambdaError};
use tracing::{error, info};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_xray::Client as XRayClient;

mod api;
mod db;
mod dicom;
mod models;
mod routes;
mod s3;
mod telemetry;

use api::request::{Request, extract_method_and_path};
use api::response::options_response;

/// Main Lambda handler function
async fn function_handler(event: LambdaEvent<Request>) -> Result<api::response::Response, LambdaError> {
    info!("FULL EVENT DUMP: {:?}", event);
    
    // Initialize AWS clients
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);
    let xray_client = XRayClient::new(&config);

    // Send X-Ray trace for request start
    telemetry::send_xray_trace(&xray_client, "request-start").await;
    
    // Extract method and path from request
    let (http_method, path) = extract_method_and_path(&event.payload);
    
    info!("PROCESSED REQUEST: method={}, path={}", http_method, path);

    // Handle OPTIONS request with proper CORS headers
    if http_method == "OPTIONS" {
        return Ok(options_response());
    }

    // Route the request
    let result = if !path.starts_with("/api") {
        // Serve frontend files
        routes::frontend::serve_frontend(&s3_client, &path).await
    } else {
        // Handle API routes based on method and path
        match (http_method.as_str(), path.as_str()) {
            // Case-related routes
            ("GET", "/api/cases") => 
                routes::cases::list_cases(&dynamodb_client).await,
                
            ("GET", p) if p.starts_with("/api/cases/") => 
                routes::cases::get_case(&dynamodb_client, p).await,
                
            ("POST", "/api/cases") => 
                routes::cases::create_case(&dynamodb_client, &s3_client, &xray_client, &event.payload.body).await,
                
            ("POST", p) if p.starts_with("/api/cases/") && p.contains("/images") => 
                routes::cases::add_images(&dynamodb_client, &s3_client, &xray_client, p, &event.payload.body).await,
            
            // DICOM-related routes
            ("GET", p) if p.starts_with("/api/dicom/") => 
                routes::dicom_routes::get_dicom(&dynamodb_client, &s3_client, &xray_client, p).await,
            
            // Not found
            _ => {
                error!("Route not found: {} {}", http_method, path);
                api::response::not_found("Route not found")
            }
        }
    };
    
    // Send X-Ray trace for request end
    telemetry::send_xray_trace(&xray_client, "request-end").await;
    
    result
}

/// Entry point for the Lambda function
#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize X-Ray
    telemetry::init_xray();
    info!("X-Ray tracing initialized");

    // Set up AWS clients
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let s3_client = S3Client::new(&config);
    let xray_client = XRayClient::new(&config);
    
    // Send X-Ray trace for Lambda startup
    telemetry::send_xray_trace(&xray_client, "lambda-startup").await;

    // Ensure resources exist
    if let Err(err) = db::ensure_table_exists(&dynamodb_client).await {
        error!("Failed to ensure DynamoDB table exists: {:?}", err);
    }

    if let Err(err) = s3::ensure_bucket_exists(&s3_client).await {
        error!("Failed to ensure S3 bucket exists: {:?}", err);
    }

    // Run the Lambda service
    info!("Starting Lambda service with X-Ray tracing enabled");
    run(service_fn(function_handler)).await
}