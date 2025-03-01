use anyhow::{Context, Result};
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::collections::HashMap;
use tracing::{info, error};

use crate::models::{Case, SeriesInfo};

// The name of the DynamoDB table
const TABLE_NAME: &str = "RadiologyTeachingFiles";

/// Save a case to DynamoDB
pub async fn save_case(client: &Client, case: &Case) -> Result<()> {
    info!("Saving case to DynamoDB: {}", case.case_id);
    
    // Convert tags to attribute values
    let tags: Vec<AttributeValue> = case.tags.iter()
        .map(|tag| AttributeValue::S(tag.clone()))
        .collect();
    
    // Convert image_ids to attribute values
    let image_ids: Vec<AttributeValue> = case.image_ids.iter()
        .map(|id| AttributeValue::S(id.clone()))
        .collect();

    // Convert series to attribute values
    let series: Vec<AttributeValue> = case.series.iter()
        .map(|series_info| {
            let mut map = HashMap::new();
            map.insert("series_instance_uid".to_string(), AttributeValue::S(series_info.series_instance_uid.clone()));
            map.insert("series_number".to_string(), AttributeValue::N(series_info.series_number.to_string()));
            map.insert("series_description".to_string(), AttributeValue::S(series_info.series_description.clone()));
            map.insert("modality".to_string(), AttributeValue::S(series_info.modality.clone()));
            
            // Convert series image_ids to attribute values
            let series_image_ids: Vec<AttributeValue> = series_info.image_ids.iter()
                .map(|id| AttributeValue::S(id.clone()))
                .collect();
            map.insert("image_ids".to_string(), AttributeValue::L(series_image_ids));
            
            AttributeValue::M(map)
        })
        .collect();

    let result = client.put_item()
        .table_name(TABLE_NAME)
        // Base case fields
        .item("case_id", AttributeValue::S(case.case_id.clone()))
        .item("title", AttributeValue::S(case.title.clone()))
        .item("description", AttributeValue::S(case.description.clone()))
        .item("modality", AttributeValue::S(case.modality.clone()))
        .item("anatomy", AttributeValue::S(case.anatomy.clone()))
        .item("diagnosis", AttributeValue::S(case.diagnosis.clone()))
        .item("findings", AttributeValue::S(case.findings.clone()))
        .item("tags", AttributeValue::L(tags))
        .item("image_ids", AttributeValue::L(image_ids))
        .item("created_at", AttributeValue::S(case.created_at.clone()))
        
        // DICOM metadata fields
        .item("study_instance_uid", AttributeValue::S(case.study_instance_uid.clone()))
        .item("series_instance_uid", AttributeValue::S(case.series_instance_uid.clone()))
        .item("study_date", AttributeValue::S(case.study_date.clone()))
        .item("study_description", AttributeValue::S(case.study_description.clone()))
        .item("patient_id", AttributeValue::S(case.patient_id.clone()))
        .item("patient_name", AttributeValue::S(case.patient_name.clone()))
        
        // Series information
        .item("series", AttributeValue::L(series))
        
        .send()
        .await
        .context("Failed to save case to DynamoDB")?;
    
    info!("Case saved successfully: {:?}", result);
    Ok(())
}

/// Get a case from DynamoDB by ID
pub async fn get_case(client: &Client, case_id: &str) -> Result<Option<Case>> {
    info!("Getting case from DynamoDB: {}", case_id);
    
    let result = client.get_item()
        .table_name(TABLE_NAME)
        .key("case_id", AttributeValue::S(case_id.to_string()))
        .send()
        .await
        .context("Failed to get case from DynamoDB")?;
    
    if let Some(item) = result.item {
        Ok(Some(convert_item_to_case(item)?))
    } else {
        info!("Case not found: {}", case_id);
        Ok(None)
    }
}

/// List all cases from DynamoDB
pub async fn list_cases(client: &Client) -> Result<Vec<Case>> {
    info!("Listing all cases from DynamoDB");
    
    let result = client.scan()
        .table_name(TABLE_NAME)
        .send()
        .await
        .context("Failed to list cases from DynamoDB")?;
    
    let mut cases = Vec::new();
    
    if let Some(items) = result.items {
        for item in items {
            match convert_item_to_case(item) {
                Ok(case) => cases.push(case),
                Err(err) => error!("Failed to convert item to case: {:?}", err),
            }
        }
    }
    
    info!("Retrieved {} cases", cases.len());
    Ok(cases)
}

/// Convert a DynamoDB item to a Case
fn convert_item_to_case(item: HashMap<String, AttributeValue>) -> Result<Case> {
    // Extract required fields
    let case_id = item.get("case_id")
        .and_then(|v| v.as_s().ok())
        .context("Missing or invalid case_id")?
        .clone();
    
    let title = item.get("title")
        .and_then(|v| v.as_s().ok())
        .context("Missing or invalid title")?
        .clone();
    
    // Fix for &String issues - use map_or to handle type correctly
    let description = item.get("description")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let modality = item.get("modality")
        .and_then(|v| v.as_s().ok())
        .map_or("Unknown".to_string(), |s| s.to_string());
    
    let anatomy = item.get("anatomy")
        .and_then(|v| v.as_s().ok())
        .map_or("Unknown".to_string(), |s| s.to_string());
    
    let diagnosis = item.get("diagnosis")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let findings = item.get("findings")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    // Extract tags
    let tags = item.get("tags")
        .and_then(|v| v.as_l().ok())
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_s().ok().cloned())
                .collect()
        })
        .unwrap_or_default();
    
    // Extract image_ids
    let image_ids = item.get("image_ids")
        .and_then(|v| v.as_l().ok())
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_s().ok().cloned())
                .collect()
        })
        .unwrap_or_default();
    
    let created_at = item.get("created_at")
        .and_then(|v| v.as_s().ok())
        .unwrap_or(&chrono::Utc::now().to_rfc3339())
        .clone();
    
    // Extract DICOM metadata fields
    let study_instance_uid = item.get("study_instance_uid")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let series_instance_uid = item.get("series_instance_uid")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let study_date = item.get("study_date")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let study_description = item.get("study_description")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let patient_id = item.get("patient_id")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    let patient_name = item.get("patient_name")
        .and_then(|v| v.as_s().ok())
        .map_or(String::new(), |s| s.to_string());
    
    // Extract series information
    let series = item.get("series")
        .and_then(|v| v.as_l().ok())
        .map(|list| {
            list.iter()
                .filter_map(|v| {
                    if let Ok(map) = v.as_m() {
                        let series_instance_uid = map.get("series_instance_uid")
                            .and_then(|v| v.as_s().ok())
                            .map_or(String::new(), |s| s.to_string());
                        
                        let series_number = map.get("series_number")
                            .and_then(|v| v.as_n().ok())
                            .and_then(|n| n.parse::<i32>().ok())
                            .unwrap_or(0);
                        
                        let series_description = map.get("series_description")
                            .and_then(|v| v.as_s().ok())
                            .map_or(String::new(), |s| s.to_string());
                        
                        let modality = map.get("modality")
                            .and_then(|v| v.as_s().ok())
                            .map_or("Unknown".to_string(), |s| s.to_string());
                        
                        let image_ids = map.get("image_ids")
                            .and_then(|v| v.as_l().ok())
                            .map(|list| {
                                list.iter()
                                    .filter_map(|v| v.as_s().ok().cloned())
                                    .collect()
                            })
                            .unwrap_or_default();
                        
                        Some(SeriesInfo {
                            series_instance_uid,
                            series_number,
                            series_description,
                            modality,
                            image_ids,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    
    Ok(Case {
        case_id,
        title,
        description,
        modality,
        anatomy,
        diagnosis,
        findings,
        tags,
        image_ids,
        created_at,
        
        // DICOM metadata fields
        study_instance_uid,
        series_instance_uid,
        study_date,
        study_description,
        patient_id,
        patient_name,
        
        // Series information
        series,
    })
}

/// Create the DynamoDB table if it doesn't exist
pub async fn ensure_table_exists(client: &Client) -> Result<()> {
    info!("Ensuring DynamoDB table exists: {}", TABLE_NAME);

    // Check if the table already exists
    match client.describe_table().table_name(TABLE_NAME).send().await {
        Ok(_) => {
            info!("Table already exists: {}", TABLE_NAME);
            Ok(())
        }
        Err(err) => {
            if err.to_string().contains("ResourceNotFoundException") {
                // Create the table
                info!("Creating table: {}", TABLE_NAME);

                use aws_sdk_dynamodb::types::{
                    AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType, BillingMode,
                };

                let key_schema = KeySchemaElement::builder()
                    .attribute_name("case_id")
                    .key_type(KeyType::Hash)
                    .build(); // No `.context()` needed

                let attribute_def = AttributeDefinition::builder()
                    .attribute_name("case_id")
                    .attribute_type(ScalarAttributeType::S)
                    .build(); // No `.context()` needed

                client.create_table()
                    .table_name(TABLE_NAME)
                    .key_schema(key_schema)
                    .attribute_definitions(attribute_def)
                    .billing_mode(BillingMode::PayPerRequest)
                    .send()
                    .await
                    .context("Failed to create DynamoDB table")?;

                info!("Table created successfully: {}", TABLE_NAME);

                // Wait for the table to become active
                info!("Waiting for table to become active...");
                let mut attempts = 0;
                let max_attempts = 10;

                while attempts < max_attempts {
                    match client.describe_table().table_name(TABLE_NAME).send().await {
                        Ok(response) => {
                            if let Some(table) = response.table() {
                                if let Some(status) = table.table_status() {
                                    if status.as_str() == "ACTIVE" {
                                        info!("Table is now active");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            info!("Error while waiting for table: {:?}, will retry", err);
                        }
                    }

                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }

                info!("Table creation initiated, but exceeded max wait time");
                Ok(())
            } else {
                Err(anyhow::anyhow!("Error checking if table exists: {:?}", err))
            }
        }
    }
}