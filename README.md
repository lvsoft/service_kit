# Service Kit: An All-in-One Rust Microservice Development Toolkit

[中文文档 (Chinese Document)](README.cn.md)

`service_kit` is a tailor-made, all-in-one development toolkit for Rust microservices. Its core goal is to **solidify best practices into tools and automate repetitive work**, allowing developers to focus on implementing core business logic.

By introducing `service_kit`, we aim to establish a standardized microservice development paradigm, ensuring that all services maintain a high degree of consistency in API specifications, code quality, type safety, and development workflows.

## Features and Composition

`service_kit` exposes several opt-in features so you can compose your service without the library taking over your Axum setup:

- **macros (default)**: Enables `#[api]` and `#[api_dto]` macros re-exported from `service_kit_macros`.
- **cli-core**: Lightweight CLI builder and completion (compatible with native/WASM environments).
- **api-cli**: Full native API CLI support (adds `tokio`, `reqwest`, terminal deps).
- **mcp**: Enables MCP router generation utilities.

Typical usage in your service (pseudocode):

```rust
let openapi = service_kit::openapi_utils::build_openapi_basic("My Service", env!("CARGO_PKG_VERSION"), "desc", "App");
let rest = service_kit::bootstrap::rest_router_from_openapi(openapi.clone())?;

// Merge into your own Axum app instead of service_kit taking over
let app = Router::new().merge(rest);
```

When `mcp` is enabled:

```rust
let tool_router = service_kit::bootstrap::mcp_router_from_openapi::<MyState>(openapi.clone())?;
let mcp_server = MyMcpServer::new(tool_router);
let mcp_service = StreamableHttpService::new(move || Ok(mcp_server.clone()), LocalSessionManager::default().into(), Default::default());
let app = app.nest_service("/mcp", mcp_service);
```

## Core Components

`service_kit` consists of three main core components:

### 1. `#[api_dto]` Procedural Macro

This is the soul of `service_kit`. By simply adding `#[api_dto]` to a Data Transfer Object (DTO) struct, you automatically get:

-   **`serde`** serialization/deserialization capabilities (`Serialize`, `Deserialize`).
-   **`utoipa`** OpenAPI Schema generation (`ToSchema`).
-   Common debugging and cloning capabilities (`Debug`, `Clone`).
-   **Built-in solution for recursion**: Automatically handles recursive types like `Box<Self>`, preventing `utoipa` compilation failures.
-   **Flexible customization**: Supports overriding naming conventions with `#[api_dto(rename_all = "...")]` and global configuration via `Cargo.toml`.

### 2. `forge_cli` & `forge-cli` Integrated Build Tools

`service_kit` provides a powerful suite of command-line tools to encapsulate the entire development, testing, and interaction workflow.
   
-   **`forge_cli`**: Built into the `service_kit` dependency and invoked via the `cargo forge` alias, it provides build and quality assurance commands:
    -   `cargo forge generate-types`: Generates TypeScript definitions from a running service's OpenAPI specification.
    -   `cargo forge lint`: Performs strict code quality checks on the project using `cargo clippy`.
    -   `cargo forge test`: Runs all unit and integration tests within the project.
-   **`forge-cli`**: A standalone, dynamic API client for interacting with your service's API.
   
### 3. `service-template` Service Template
   
A standard `cargo-generator` template that allows developers to quickly initialize a new microservice project skeleton conforming to the `service_kit` specification with a single command.
   
---
   
## Getting Started Guide
   
This guide will walk you through creating and running your first `service_kit` microservice.
   
### Step 1: Install Prerequisites
   
You need to install `cargo-generate` and `openapi-typescript`.
   
```bash
# Install the project template generator
cargo install cargo-generate

# Install the OpenAPI to TypeScript converter
npm install -g openapi-typescript
```
   
### Step 2: Create a New Service from the Template
   
Use the `cargo generate` command to create a new project named `my-awesome-service` from the Git repository.
   
```bash
# This command clones the service_kit repository from GitHub and uses the service-template directory as the template
cargo generate --git https://github.com/lvsoft/service_kit.git service-template --name my-awesome-service
```
   
### Step 3: Run the Service (feature toggles)
   
Navigate into the newly created project directory and start the service.
   
```bash
cd my-awesome-service
## default (all on in the template): swagger-ui, wasm-cli, mcp
cargo run

## turn off all template features
cargo run --no-default-features

## selectively enable
cargo run --no-default-features --features swagger-ui
cargo run --no-default-features --features wasm-cli
cargo run --no-default-features --features mcp
```
   
---
   
## `forge` Command Demonstration
   
### `cargo forge` (Build & Quality)

By default the template wires a cargo alias so `cargo forge` resolves to the external `forge-cli` binary. To use it:

1) Install the binary (once):
```bash
cargo install service_kit --features api-cli
```

2) In a project generated from the template, run:
```bash
cargo forge help
```

All `cargo forge` commands should be run from within **your generated service directory** (e.g., `my-awesome-service/`).
   
-   **`cargo forge test`**: Runs all tests for the project.
-   **`cargo forge lint`**: Performs strict code quality checks on the project.
-   **`cargo forge generate-types`**: Generates a TypeScript file from your running service's OpenAPI spec.
    
    **Prerequisite**: Ensure your service is running in another terminal (`cargo run`).
    
    ```bash
    # Usage: cargo forge generate-types --input <URL_TO_OPENAPI_JSON> --output <PATH_TO_TS_FILE>
    cargo forge generate-types --input http://127.0.0.1:3000/api-docs/openapi.json --output src/frontend/types/api.ts
    ```

### `forge-cli` (API Client)

`service_kit` provides a binary named `forge-cli`, which is an interactive API client based on the OpenAPI specification.

**Installation**:
Install from crates.io and enable the `api-cli` feature flag.

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
