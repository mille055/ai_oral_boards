# Introduction

radiology-teaching-files is a Rust project that implements an AWS Lambda function in Rust.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## Building

To build the project for production, run `cargo lambda build --release`. Remove the `--release` flag to build for development.

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`.

If you want to run integration tests locally, you can use the `cargo lambda watch` and `cargo lambda invoke` commands to do it.

First, run `cargo lambda watch` to start a local server. When you make changes to the code, the server will automatically restart.

Second, you'll need a way to pass the event data to the lambda function.

You can use the existent [event payloads](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) in the Rust Runtime repository if your lambda function is using one of the supported event types.

You can use those examples directly with the `--data-example` flag, where the value is the name of the file in the [lambda-events](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) repository without the `example_` prefix and the `.json` extension.

```bash
cargo lambda invoke --data-example apigw-request
```

For generic events, where you define the event data structure, you can create a JSON file with the data you want to test with. For example:

```json
{
    "command": "test"
}
```

Then, run `cargo lambda invoke --data-file ./data.json` to invoke the function with the data in `data.json`.

For HTTP events, you can also call the function directly with cURL or any other HTTP client. For example:

```bash
curl https://localhost:9000
```

Read more about running the local server in [the Cargo Lambda documentation for the `watch` command](https://www.cargo-lambda.info/commands/watch.html).
Read more about invoking the function in [the Cargo Lambda documentation for the `invoke` command](https://www.cargo-lambda.info/commands/invoke.html).

## Deploying

To deploy the project, run `cargo lambda deploy`. This will create an IAM role and a Lambda function in your AWS account.

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).


# Radiology Teaching Files - Rust Lambda Microservice

## 📌 Overview
This project is a **Rust-based AWS Lambda microservice** that processes radiology teaching files. It includes a **frontend** (React) and a **backend** built in Rust, designed to run efficiently on **AWS Lambda**.

### Features:
- 🚀 **Serverless Rust microservice** for efficient handling of radiology files.
- ⚡ **Optimized CI/CD pipeline** for automated deployment.
- ☁️ **AWS integration**: S3 for storage, DynamoDB for metadata.
- 🛠️ **Rust toolchain**: Uses `cargo lambda` for building and deploying AWS Lambda functions.

## 🛠️ Prerequisites
Before setting up, ensure you have:
- 🦀 [Rust](https://www.rust-lang.org/) (with `cargo`)
- 📦 [Cargo Lambda](https://github.com/cargo-lambda/cargo-lambda) (`cargo install cargo-lambda`)
- 🏗️ [Docker](https://www.docker.com/)
- ☁️ AWS CLI configured with appropriate credentials
- 🖥️ GitLab CI/CD setup (if using automated deployment)

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

### **Pipeline Stages:**
1. **Build Frontend** (Node.js) → `npm run build`
2. **Build Backend** (Rust Lambda) → `cargo lambda build`
3. **Deploy** → `cargo lambda deploy`

### **Pipeline Configuration (`.gitlab-ci.yml`)**
```yaml
stages:
  - build
  - deploy

deploy:
  stage: deploy
  image: ghcr.io/cargo-lambda/cargo-lambda:latest
  script:
    - cargo lambda deploy --iam-role $AWS_LAMBDA_ROLE --region $AWS_DEFAULT_REGION $LAMBDA_FUNCTION_NAME
  dependencies:
    - build-backend
  only:
    - main
  environment:
    name: production
```

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

