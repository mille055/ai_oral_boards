use aws_sdk_xray::Client as XRayClient;
use chrono::Utc;
use uuid::Uuid;

/// Initialize X-Ray environment variables
pub fn init_xray() {
    if std::env::var("AWS_XRAY_DAEMON_ADDRESS").is_err() {
        std::env::set_var("AWS_XRAY_DAEMON_ADDRESS", "127.0.0.1:2000");
    }
    
    if std::env::var("AWS_XRAY_CONTEXT_MISSING").is_err() {
        std::env::set_var("AWS_XRAY_CONTEXT_MISSING", "LOG_ERROR");
    }
    
    // Using println since the tracing may not be initialized yet
    println!("X-Ray environment variables configured");
}

/// Send an X-Ray trace segment
pub async fn send_xray_trace(xray_client: &XRayClient, name: &str) {
    let timestamp = Utc::now().timestamp_millis() as f64 / 1000.0;
    let segment_id = Uuid::new_v4().to_string().chars().take(16).collect::<String>();

    let trace_segment = format!(
        r#"{{
            "name": "{}",
            "id": "{}",
            "start_time": {},
            "end_time": {},
            "in_progress": false,
            "service": {{
                "version": "1.0.0",
                "name": "radiology-teaching-files"
            }}
        }}"#, 
        name,
        segment_id,
        timestamp,
        timestamp + 0.001
    );

    match xray_client.put_trace_segments()
        .trace_segment_documents(trace_segment)
        .send().await {
        Ok(_) => println!("X-Ray trace sent for {}", name),
        Err(e) => eprintln!("Failed to send X-Ray trace for {}: {:?}", name, e),
    }
}