use anyhow::{Context, Result};
use dicom_object::open_file;
use std::path::Path;
use tracing::info;

use crate::models::DicomMetadata;

/// Extract metadata from a DICOM file
pub fn extract_metadata(data: &[u8]) -> Result<DicomMetadata> {
    info!("Extracting metadata from DICOM data");
    
    // dicom_object crate doesn't have direct memory loading in older versions
    // We need to write the data to a temporary file and then read it back
    let temp_file = tempfile::NamedTempFile::new()
        .context("Failed to create temporary file")?;
    let temp_path = temp_file.path().to_owned();
    
    // Write the data to the temporary file
    std::fs::write(&temp_path, data)
        .context("Failed to write DICOM data to temporary file")?;
    
    // Extract metadata from the file directly
    extract_metadata_from_file(&temp_path)
}

/// Extract metadata from a DICOM file on disk
pub fn extract_metadata_from_file<P: AsRef<Path>>(path: P) -> Result<DicomMetadata> {
    info!("Extracting metadata from DICOM file: {:?}", path.as_ref());
    
    // Open the DICOM file
    let obj = open_file(path.as_ref())
        .context("Failed to open DICOM file")?;
    
    // Extract required fields first, these will error if missing
    let element = obj.element_by_name("SOPInstanceUID")
        .context("Missing SOPInstanceUID")?;
    let sop_instance_uid = match element.to_str() {
        Ok(value) => value.to_string(),
        Err(e) => return Err(anyhow::anyhow!("Invalid SOPInstanceUID format: {:?}", e))
    };
    
    let element = obj.element_by_name("StudyInstanceUID")
        .context("Missing StudyInstanceUID")?;
    let study_instance_uid = match element.to_str() {
        Ok(value) => value.to_string(),
        Err(e) => return Err(anyhow::anyhow!("Invalid StudyInstanceUID format: {:?}", e))
    };
    
    let element = obj.element_by_name("SeriesInstanceUID")
        .context("Missing SeriesInstanceUID")?;
    let series_instance_uid = match element.to_str() {
        Ok(value) => value.to_string(),
        Err(e) => return Err(anyhow::anyhow!("Invalid SeriesInstanceUID format: {:?}", e))
    };
    
    let element = obj.element_by_name("Modality")
        .context("Missing Modality")?;
    let modality = match element.to_str() {
        Ok(value) => value.to_string(),
        Err(e) => return Err(anyhow::anyhow!("Invalid Modality format: {:?}", e))
    };
    
    // Optional fields - use defaults if missing
    let patient_name = match obj.element_by_name("PatientName") {
        Ok(element) => {
            match element.to_str() {
                Ok(value) => value.to_string(),
                Err(_) => "Anonymous".to_string()
            }
        },
        Err(_) => "Anonymous".to_string()
    };
    
    let patient_id = match obj.element_by_name("PatientID") {
        Ok(element) => match element.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => "Unknown".to_string()
        },
        Err(_) => "Unknown".to_string()
    };
    
    let study_date = match obj.element_by_name("StudyDate") {
        Ok(element) => match element.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => "Unknown".to_string()
        },
        Err(_) => "Unknown".to_string()
    };
    
    let study_description = match obj.element_by_name("StudyDescription") {
        Ok(element) => match element.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => "Unknown".to_string()
        },
        Err(_) => "Unknown".to_string()
    };
    
    let series_description = match obj.element_by_name("SeriesDescription") {
        Ok(element) => match element.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => "Unknown".to_string()
        },
        Err(_) => "Unknown".to_string()
    };
    
    let instance_number = match obj.element_by_name("InstanceNumber") {
        Ok(element) => element.to_int::<i32>().unwrap_or(0),
        Err(_) => 0
    };
    
    info!("Extracted DICOM metadata: SOPInstanceUID={}", sop_instance_uid);
    
    Ok(DicomMetadata {
        sop_instance_uid,
        study_instance_uid,
        series_instance_uid,
        modality,
        patient_name,
        patient_id,
        study_date,
        study_description,
        series_description,
        instance_number,
    })
}