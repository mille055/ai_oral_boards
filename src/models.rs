use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Case {
    pub case_id: String,
    pub title: String,
    pub description: String,
    pub modality: String,
    pub anatomy: String,
    pub diagnosis: String,
    pub findings: String,
    pub tags: Vec<String>,
    pub image_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaseUpload {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub modality: String,  // Ensure modality is included
    pub anatomy: String,
    pub diagnosis: String,
    pub findings: String,
    pub tags: Vec<String>,
    #[serde(rename = "dicomFile")]
    pub dicom_file: String,  // Base64 encoded DICOM file
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaseMetadata {
    pub case_id: String,
    pub title: String,
    pub modality: String,
    pub anatomy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DicomMetadata {
    pub sop_instance_uid: String,
    pub study_instance_uid: String,
    pub series_instance_uid: String,
    pub modality: String,
    pub patient_name: String,
    pub patient_id: String,
    pub study_date: String,
    pub study_description: String,
    pub series_description: String,
    pub instance_number: i32,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub error_code: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }
}

impl ErrorResponse {
    pub fn not_found(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            error_code: "NOT_FOUND".to_string(),
        }
    }
    
    pub fn bad_request(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            error_code: "BAD_REQUEST".to_string(),
        }
    }
    
    pub fn server_error(message: String) -> Self {
        Self {
            success: false,
            error: message,
            error_code: "SERVER_ERROR".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn not_implemented(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            error_code: "NOT_IMPLEMENTED".to_string(),
        }
    }
}
