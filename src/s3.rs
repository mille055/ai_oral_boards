use anyhow::{Context, Result};
use aws_sdk_s3::{Client, primitives::ByteStream};
use tracing::info;
use std::env;

/// Retrieves the bucket name from environment variables or falls back to a default.
fn get_bucket_name() -> String {
    env::var("S3_BUCKET").unwrap_or_else(|_| "radiology-teaching-files".to_string())
}

/// Upload a file to S3
pub async fn upload_file(client: &Client, key: &str, data: Vec<u8>) -> Result<()> {
    let bucket_name = get_bucket_name();
    info!("Uploading file to S3: {}/{}", bucket_name, key);
    
    let len = data.len();
    let body = ByteStream::from(data);
    
    client.put_object()
        .bucket(&bucket_name)
        .key(key)
        .body(body)
        .content_type("application/dicom")
        .send()
        .await
        .context(format!("Failed to upload file to S3 at {}/{}", bucket_name, key))?;
    
    info!("File uploaded successfully: {} ({} bytes)", key, len);
    Ok(())
}

/// Download a file from S3
pub async fn download_file(client: &Client, key: &str) -> Result<Vec<u8>> {
    let bucket_name = get_bucket_name();
    info!("Downloading file from S3: {}/{}", bucket_name, key);
    
    let result = client.get_object()
        .bucket(&bucket_name)
        .key(key)
        .send()
        .await
        .context(format!("Failed to download file from S3 at {}/{}", bucket_name, key))?;
    
    let data = result.body.collect().await?;
    let bytes = data.into_bytes().to_vec();
    
    info!("File downloaded successfully: {} ({} bytes)", key, bytes.len());
    Ok(bytes)
}

/// Check if a file exists in S3
#[allow(dead_code)]
pub async fn file_exists(client: &Client, key: &str) -> Result<bool> {
    let bucket_name = get_bucket_name();
    info!("Checking if file exists in S3: {}/{}", bucket_name, key);
    
    let result = client.head_object()
        .bucket(&bucket_name)
        .key(key)
        .send()
        .await;
    
    match result {
        Ok(_) => {
            info!("File exists: {}", key);
            Ok(true)
        },
        Err(err) => {
            if err.to_string().contains("NotFound") {
                info!("File does not exist: {}", key);
                Ok(false)
            } else {
                Err(anyhow::anyhow!("Error checking if file exists: {:?}", err))
            }
        }
    }
}

/// Ensure the S3 bucket exists (only create if necessary)
pub async fn ensure_bucket_exists(client: &Client) -> Result<()> {
    let bucket_name = get_bucket_name();
    info!("Ensuring S3 bucket exists: {}", bucket_name);
    
    let result = client.head_bucket()
        .bucket(&bucket_name)
        .send()
        .await;
    
    match result {
        Ok(_) => {
            info!("Bucket already exists: {}", bucket_name);
            Ok(())
        },
        Err(err) => {
            if err.to_string().contains("NotFound") {
                info!("Bucket does not exist. Creating: {}", bucket_name);
                
                client.create_bucket()
                    .bucket(&bucket_name)
                    .send()
                    .await
                    .context(format!("Failed to create S3 bucket: {}", bucket_name))?;
                
                info!("Bucket created successfully: {}", bucket_name);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Error checking if bucket exists: {:?}", err))
            }
        }
    }
}
