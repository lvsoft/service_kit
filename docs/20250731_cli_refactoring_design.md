# `forge-api-cli` 模块化重构设计文档

## 1. 背景与动机 (Background & Motivation)

`service_kit` 的开发过程经过了多次迭代。在引入动态 API 客户端 (`api-cli`) 的过程中，为了快速实现功能并保持逻辑分离，我们将其创建为一个独立的 `forge-api-cli` Crate。

当前架构如下:
- `service_kit`: 提供 `#[api_dto]` 宏和 `forge-cli` (构建/测试工具)。
- `forge-api-cli`: 一个独立的**库** Crate，包含所有 API 客户端的复杂逻辑。
- `service-template`: 生成的项目需要**同时依赖** `service_kit` 和 `forge-api-cli` 这两个 Crate。

这个架构虽然能够工作，但存在两个核心问题:

1.  **逻辑分散**: `forge-api-cli` 本质上是 `service_kit` 工具生态系统的一部分，将它作为一个独立的、必须由用户单独依赖的 Crate，在逻辑上是不统一的。这增加了用户的认知负担。
2.  **依赖冗余**: 生成的模板项目需要声明两个来自同一代码仓库的依赖，这显得很冗余，也增加了版本管理的复杂性。

本次重构旨在解决这些问题，将 `api-cli` 的功能以内聚、模块化的方式回归到 `service_kit` Crate 内部。

## 2. 目标 (Goals)

-   **逻辑统一**: 将 `forge-api-cli` 的所有功能逻辑作为 `service_kit` Crate 的一个可选模块。
-   **依赖简化**: 将生成的模板项目的依赖从两个 (`service_kit`, `forge-api-cli`) 简化为**一个** (`service_kit`)。
-   **按需编译 (On-demand Compilation)**: 利用 Cargo 的 `feature` flag 机制，使得 `api-cli` 模块及其所有重量级依赖 (如 `clap`, `reqwest`, `reedline` 等) 只有在用户需要时才被编译，从而优化非 CLI 场景下的编译速度和依赖树。

## 3. 详细设计方案 (Detailed Design)

### 3.1. 项目结构变更

我们将移除独立的 `forge-api-cli` Crate，并将其源代码整合进 `service_kit`。

**重构前:**
```
.
├── forge-api-cli/
│   ├── src/
│   │   ├── cli.rs
│   │   ├── client.rs
│   │   ├── ...
│   │   └── lib.rs
│   └── Cargo.toml
└── service_kit/
    └── ...
```

**重构后:**
```
.
└── service_kit/
    ├── src/
    │   ├── api_cli/      <-- 新的模块目录
    │   │   ├── mod.rs
    │   │   ├── cli.rs
    │   │   ├── client.rs
    │   │   └── ...
    │   ├── lib.rs        <-- Proc-macro a
    │   └── main.rs       <-- forge-cli a
    └── Cargo.toml
```

### 3.2. `service_kit/Cargo.toml` 修改

`service_kit` 的 `Cargo.toml` 将被大幅修改，以包含 `api-cli` 的依赖并定义 `cli` feature。

```toml
# In service_kit/Cargo.toml

[package]
# ...

[lib]
proc-macro = true

[[bin]]
name = "forge-cli"
path = "src/main.rs"
# 只有当 "cli" feature 被启用时，我们才需要编译这个二进制文件
required-features = ["cli-support"]

[dependencies]
# Proc-macro dependencies...
syn = { ... }
quote = "1.0"
# ...

# CLI dependencies (now optional)
anyhow = { version = "1.0", optional = true }
clap = { ..., optional = true }
reqwest = { ..., optional = true }
reedline = { ..., optional = true }
# ... (所有原 forge-api-cli 的依赖)

[features]
default = []
# "cli-support" feature 将启用 forge-cli 二进制文件本身
cli-support = ["dep:anyhow", "dep:clap", "dep:toml"]
# "api-cli" feature 启用完整的 API 客户端逻辑及其所有依赖
api-cli = [
    "cli-support", 
    "dep:reqwest", 
    "dep:reedline", 
    "dep:oas", 
    # ... etc
]
```
**注意**: 我们将 `forge-cli` 和 `api-cli` 的功能分离到不同的 feature 中，以获得更精细的控制。

### 3.3. `service_kit` 代码修改

- **`service_kit/src/lib.rs` (proc-macro 库)**:
    - 我们将把 `api_cli` 的所有逻辑放入一个新的 `api_cli` 模块中。
    - 整个模块将由 `cli` feature 控制。
  ```rust
  // In service_kit/src/lib.rs
  
  // ... proc-macro code ...
  
  #[cfg(feature = "api-cli")]
  pub mod api_cli;
  ```

- **`service_kit/src/main.rs` (`forge-cli` 二进制文件)**:
    - `api-cli` 子命令的实现将不再是通过 `cargo run --bin ...` 的代理，而是直接调用 `service_kit::api_cli::run()` 函数。
  ```rust
  // In service_kit/src/main.rs
  
  // ... clap setup ...
  
  fn main() -> Result<()> {
      // ...
      match cli.command {
          Commands::ApiCli(args) => {
              // 直接调用，但需要确保是在启用了 "api-cli" feature 的情况下编译的
              #[cfg(feature = "api-cli")]
              service_kit::api_cli::run_with_args(args)?;

              #[cfg(not(feature = "api-cli"))]
              println!("'api-cli' feature is not enabled.");
          }
          // ...
      }
      Ok(())
  }
  ```

### 3.4. `service-template` 修改

模板的修改是本次重构的核心收益点。

- **`service-template/Cargo.toml`**:
    - 依赖将从两个锐减为一个。
    - `api-cli` 的功能通过 `service_kit` 的 `features` 来启用。
  ```toml
  # In service-template/Cargo.toml
  [dependencies]
  service_kit = { git = "...", rev = "...", features = ["api-cli"] }
  anyhow = "1.0"
  # ... (不再需要 forge-api-cli 依赖)

  # ...
  [[bin]]
  name = "api-cli"
  # 只有需要时才编译
  required-features = ["service_kit/api-cli"] 
  ```

- **`service-template/src/bin/api-cli.rs`**:
    - 入口文件现在直接调用 `service_kit` 中的函数。
  ```rust
  // in service-template/src/bin/api-cli.rs
  fn main() -> anyhow::Result<()> {
      // 只有在 feature 启用时才可用
      #[cfg(feature = "api-cli")]
      return service_kit::api_cli::run();

      #[cfg(not(feature = "api-cli"))]
      panic!("api-cli requires the 'api-cli' feature to be enabled in service_kit.");
  }
  ```

## 4. 最终用户影响

- **依赖更简洁**: 新项目只需要维护一个指向 `service_kit` 的依赖。
- **使用体验不变**: `cargo forge api-cli ...` 的命令和用法保持完全不变。
- **性能优化**: 对于不需要 CLI 工具的场景（例如在 CI 中只运行 `cargo build`），可以通过在 `Cargo.toml` 中移除 `features = ["api-cli"]` 来显著减少编译时间和依赖项数量。
