// // tests/integration-tests.rs

// use reqwest;
// use serde_json::json;
// use tokio;

// #[tokio::test]
// async fn test_get_cases_api() {
//     let client = reqwest::Client::new();
//     let res = client.get("https://pvhymfafqoym6f7uj4wj4dzsh40bprml.lambda-url.us-east-1.on.aws/api/cases")
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(res.status(), 200);
//     let json: serde_json::Value = res.json().await.unwrap();
//     assert!(json["success"].as_bool().unwrap());
// }

// #[tokio::test]
// async fn test_post_case_api() {
//     let client = reqwest::Client::new();
//     let payload = json!({
//         "title": "Test Case",
//         "description": "This is a test case",
//         "modality": "CT",
//         "anatomy": "Brain",
//         "diagnosis": "Normal",
//         "findings": "No issues",
//         "tags": ["test"],
//         "dicom_file": "BASE64_ENCODED_DICOM"
//     });

//     let res = client.post("https://pvhymfafqoym6f7uj4wj4dzsh40bprml.lambda-url.us-east-1.on.aws/api/cases")
//         .json(&payload)
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(res.status(), 201);
//     let json: serde_json::Value = res.json().await.unwrap();
//     assert!(json["success"].as_bool().unwrap());
// }
