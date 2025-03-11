# Radiology Teaching Files - Rust Lambda Microservice

## 📌 Overview
This project is a **Rust-based AWS Lambda microservice** that processes DICOM images with user defined tags and diagnoses to create a teaching file. It includes a **backend** built in Rust, designed to run efficiently on **AWS Lambda** as well as a frontend consisting of javascript files. It is the individual project #2 (but builds upon mini-projects 5 and 6).

### Features:
- 🚀 **Serverless Rust microservice** for efficient handling of radiology files.
- ⚡ **Optimized CI/CD pipeline** for automated deployment.
- ☁️ **AWS integration**: S3 for storage, DynamoDB for metadata.
- 🛠️ **Rust toolchain**: Uses `cargo lambda` for building and deploying AWS Lambda functions.
-  **Tracing and logging:** for Rust Lmabda. 

### Logging and Tracing
This project uses Rust’s tracing crate to provide structured logging for debugging and monitoring AWS Lambda execution. Logs are categorized into different levels (trace, debug, info, warn, and error) to capture everything from fine-grained debugging details to high-level system events. The logging system helps track Lambda requests, AWS SDK interactions (DynamoDB, S3), and performance metrics, ensuring visibility into function execution. These logs are crucial for diagnosing failures, monitoring API requests, and optimizing performance in a serverless environment. The log level can be adjusted using the RUST_LOG environment variable to filter relevant details for debugging or production monitoring.

trace: Detailed step-by-step execution logs.
debug: Logs for debugging API requests and variables.
info: General system events and successful operations.
warn: Non-fatal issues requiring attention.
error: Critical errors preventing execution.


## 🛠️ Prerequisites
Before setting up, ensure you have:
- 🦀 [Rust](https://www.rust-lang.org/) (with `cargo`)
- 📦 [Cargo Lambda](https://github.com/cargo-lambda/cargo-lambda) (`cargo install cargo-lambda`)
- 🏗️ [Docker](https://www.docker.com/)
- ☁️ AWS CLI configured with appropriate credentials
- 🖥️ GitLab CI/CD setup (if using automated deployment)

---
## Screenshots of the interface and test case in the database, showing successful update of the database and the logging information

![png](assets/cases2.png)
![png](assets/upload2.png)
![png](assets/upload2success.png)
![png](assets/dbconsole2.png)
![png](assets/logging.png)
![png](assets/tracing.png)

---
The interface can be found at [https://radiology-teaching-files.s3.amazonaws.com/frontend/index.html](https://radiology-teaching-files.s3.amazonaws.com/frontend/index.html). As of right now, the image viewer is not fully deployed,but the components for this miniproject of a rust lambda microservice communicating with a DynamoDB database has been deployed.

---
## 🚀 Setup Instructions
### 1️⃣ Clone the repository
```sh
git clone https://gitlab.com/dukeaiml/ids721-spring2025/chad-miniproject5.git
cd chad-miniproject5
```

### 2️⃣ Install Dependencies
```sh
rustup update
cargo install cargo-lambda --locked
```

### 3️⃣ Build for AWS Lambda
```sh
cargo lambda build --release --target=aarch64-unknown-linux-gnu
```

### 4️⃣ Deploy to AWS Lambda
```sh
cargo lambda deploy --iam-role <AWS_LAMBDA_ROLE> --region us-east-1 radiology-teaching-files
```

---

## 📦 CI/CD Pipeline (GitLab)
The project includes a **GitLab CI/CD pipeline** that automates build & deployment.

---

## 📂 AWS Services Used
- **AWS Lambda** - Serverless function execution.
- **Amazon S3** - File storage for radiology teaching files.
- **DynamoDB** - Metadata storage.
- **API Gateway** (Optional) - For exposing REST endpoints.

---

## 🛠️ Troubleshooting
**Issue:** CI/CD Fails at Deploy 🚨
- Ensure the deployment stage **uses the correct Docker image (`ghcr.io/cargo-lambda/cargo-lambda:latest`)**.
- Check if **AWS credentials** are configured properly (`aws sts get-caller-identity`).

**Issue:** `cargo: command not found` ❌
- Verify the **Docker image used in the CI/CD pipeline** includes `cargo` and `cargo-lambda`.

---

## ✨ Future Enhancements
- Add **unit tests** using `cargo test`
- Implement **monitoring** via AWS CloudWatch
- Extend **API Gateway support** for external integrations

---

## 📜 License
This project is open-source under the **MIT License**.

---

## 👨‍💻 Author
Dr. Chad Miller - [Duke Radiology](https://radiology.duke.edu/)

