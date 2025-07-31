# Service Kit: An All-in-One Rust Microservice Development Toolkit

[中文文档 (Chinese Document)](README.cn.md)

`service_kit` is a tailor-made, all-in-one development toolkit for Rust microservices. Its core goal is to **solidify best practices into tools and automate repetitive work**, allowing developers to focus on implementing core business logic.

By introducing `service_kit`, we aim to establish a standardized microservice development paradigm, ensuring that all services maintain a high degree of consistency in API specifications, code quality, type safety, and development workflows.

## Core Components

`service_kit` consists of three main core components:

### 1. `#[api_dto]` Procedural Macro

This is the soul of `service_kit`. By simply adding `#[api_dto]` to a Data Transfer Object (DTO) struct, you automatically get:

-   **`serde`** serialization/deserialization capabilities (`Serialize`, `Deserialize`).
-   **`utoipa`** OpenAPI Schema generation (`ToSchema`).
-   **`ts-rs`** TypeScript type definition generation (`TS`).
-   Common debugging and cloning capabilities (`Debug`, `Clone`).
-   **Built-in solution for recursion**: Automatically handles recursive types like `Box<Self>`, preventing `utoipa` compilation failures.
-   **Flexible customization**: Supports overriding naming conventions with `#[api_dto(rename_all = "...")]` and global configuration via `Cargo.toml`.

### 2. `forge_cli` & `forge-cli` Integrated Build Tools

`service_kit` provides a powerful suite of command-line tools to encapsulate the entire development, testing, and interaction workflow.

-   **`forge_cli`**: Built into the `service_kit` dependency and invoked via the `cargo forge` alias, it provides build and quality assurance commands:
    -   `cargo forge generate-ts`: Generates TypeScript definitions for all `#[api_dto]` structs.
    -   `cargo forge lint`: Performs strict code quality checks on the project using `cargo clippy`.
    -   `cargo forge test`: Runs all unit and integration tests within the project.
-   **`forge-cli`**: A standalone, dynamic API client for interacting with your service's API.

### 3. `service-template` Service Template

A standard `cargo-generator` template that allows developers to quickly initialize a new microservice project skeleton conforming to the `service_kit` specification with a single command.

---

## Getting Started Guide

This guide will walk you through creating and running your first `service_kit` microservice.

### Step 1: Install Prerequisites

You need to install `cargo-generate`.

```bash
# Install the project template generator
cargo install cargo-generate
```

### Step 2: Create a New Service from the Template

Use the `cargo generate` command to create a new project named `my-awesome-service` from the Git repository.

```bash
# This command clones the service_kit repository from GitHub and uses the service-template directory as the template
cargo generate --git https://github.com/lvsoft/service_kit.git service-template --name my-awesome-service
```

### Step 3: Run the Service

Navigate into the newly created project directory and start the service.

```bash
cd my-awesome-service
cargo run
```

---

## `forge` Command Demonstration

### `cargo forge` (Build & Quality)

All `cargo forge` commands should be run from within **your generated service directory** (e.g., `my-awesome-service/`). These commands are provided by your project's `service_kit` dependency.

-   **`cargo forge test`**: Runs all tests for the project.
-   **`cargo forge lint`**: Performs strict code quality checks on the project.
-   **`cargo forge generate-ts`**: Generates TypeScript definitions for the DTOs in your project.

### `forge-cli` (API Client)

`service_kit` provides a binary named `forge-cli`, which is an interactive API client based on the OpenAPI specification.

**Installation**:
Once `service_kit` is published to crates.io, you can install it using `cargo install`. Note that you need to enable the `api-cli` feature flag.

```bash
cargo install service_kit --features api-cli
```

**Prerequisite**: Ensure your service is running in another terminal (`cargo run`).

You can use it to call API endpoints in your service. It supports two modes:

#### 1. Direct Command Mode

For quick, one-off API calls.

```sh
# Format: forge-api-cli <BASE_URL> <API_COMMAND> [OPTIONS]
forge-api-cli http://127.0.0.1:3000 v1.hello.get
```
```json
{
  "message": "Hello, World!"
}
```

#### 2. Interactive Mode (REPL)

By providing only the URL, you can enter an interactive environment, which is ideal for API exploration and debugging.

```sh
forge-api-cli http://127.0.0.1:3000
```
```
(api-cli) > help  # Display all available commands
(api-cli) > v1.hello.get <Tab>  # Enjoy autocompletion
(api-cli) > v1.hello.get
{
  "message": "Hello, World!"
}
```

---

## Example Project

This repository includes a more comprehensive example project located at `examples/product-service`. It demonstrates the use of more complex DTOs, recursive structures, and custom naming strategies, serving as a valuable reference for your development.
