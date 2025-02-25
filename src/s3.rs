use anyhow::{Context, Result};
use aws_sdk_s3::{Client, primitives::ByteStream};
use tracing::info;

// The name of the S3 bucket
const BUCKET_NAME: &str = "radiology-teaching-files";

/// Upload a file to S3
pub async fn upload_file(client: &Client, key: &str, data: Vec<u8>) -> Result<()> {
    info!("Uploading file to S3: {}", key);
    
    let len = data.len();
    let body = ByteStream::from(data);
    
    client.put_object()
        .bucket(BUCKET_NAME)
        .key(key)
        .body(body)
        .content_type("application/dicom")
        .send()
        .await
        .context("Failed to upload file to S3")?;
    
    info!("File uploaded successfully: {} ({} bytes)", key, len);
    Ok(())
}

/// Download a file from S3
pub async fn download_file(client: &Client, key: &str) -> Result<Vec<u8>> {
    info!("Downloading file from S3: {}", key);
    
    let result = client.get_object()
        .bucket(BUCKET_NAME)
        .key(key)
        .send()
        .await
        .context("Failed to download file from S3")?;
    
    let data = result.body.collect().await?;
    let bytes = data.into_bytes().to_vec();
    
    info!("File downloaded successfully: {} ({} bytes)", key, bytes.len());
    Ok(bytes)
}

/// Check if a file exists in S3
#[allow(dead_code)]
pub async fn file_exists(client: &Client, key: &str) -> Result<bool> {
    info!("Checking if file exists in S3: {}", key);
    
    let result = client.head_object()
        .bucket(BUCKET_NAME)
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

/// Ensure the S3 bucket exists
pub async fn ensure_bucket_exists(client: &Client) -> Result<()> {
    info!("Ensuring S3 bucket exists: {}", BUCKET_NAME);
    
    // Check if the bucket already exists
    let result = client.head_bucket()
        .bucket(BUCKET_NAME)
        .send()
        .await;
    
    match result {
        Ok(_) => {
            info!("Bucket already exists: {}", BUCKET_NAME);
            Ok(())
        },
        Err(err) => {
            if err.to_string().contains("NotFound") {
                // Create the bucket
                info!("Creating bucket: {}", BUCKET_NAME);
                
                client.create_bucket()
                    .bucket(BUCKET_NAME)
                    .send()
                    .await
                    .context("Failed to create S3 bucket")?;
                
                info!("Bucket created successfully: {}", BUCKET_NAME);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Error checking if bucket exists: {:?}", err))
            }
        }
    }
}