# Service Kit 统一 Crate 设计方案

**作者**: Gemini Pro (AI Assistant)
**日期**: 2025-08-09
**状态**: **草案**

## 1. 背景与目标

当前 `service_kit` 的实现分散在三个独立的 crate 中：`service_kit`, `forge-core`, 和 `service-kit-macros`。这种结构将实现细节暴露给了最终用户，增加了使用的复杂性，并且不利于版本管理和发布。

**目标**: 将 `service_kit` 重构为一个统一的、面向用户的门面 (Facade) crate。用户应该只需要在他们的 `Cargo.toml` 中添加一个 `service_kit` 依赖，就能获得所有核心功能，包括运行时 builder 和过程宏。

## 2. 核心挑战：过程宏的限制

Rust 的一个核心限制是：一个 crate 不能同时是普通库（library）和过程宏库（`proc-macro = true`）。这意味着我们不能将 `service-kit-macros` 的代码直接合并到 `service_kit` 的 `lib.rs` 中。

## 3. 设计方案：门面 + 再导出 (Facade + Re-export)

我们将借鉴 `axum`, `serde` 等成熟框架广泛采用的模式，将 `service_kit` 设计为一个门面，它整合了运行时核心功能，并“再导出”过程宏。

### 3.1. 新的 Crate 结构

-   **`service_kit` (门面 Crate)**:
    -   这将是用户唯一需要直接依赖的 crate。
    -   它将包含原来 `forge-core` 的所有运行时逻辑。
    -   它会作为 `service-kit-macros` 的一个依赖，并通过 `pub use` 将宏导出。

-   **`service-kit-macros` (过程宏 Crate)**:
    -   保持为一个独立的 `proc-macro = true` crate。
    -   它将成为 `service_kit` crate 的一个**私有**实现细节，用户无需关心它的存在。

### 3.2. 目录结构变更

我们将移除 `forge-core` 目录，并将其内容合并到 `service_kit` 中。

**变更前:**
```
service_kit/
├── Cargo.toml
├── forge-core/
│   ├── Cargo.toml
│   └── src/
├── service-kit-macros/
│   ├── Cargo.toml
│   └── src/
└── src/
    └── lib.rs (几乎为空)
```

**变更后:**
```
service_kit/
├── Cargo.toml          # 统一的、面向用户的 Cargo.toml
├── service-kit-macros/ # 过程宏 crate 保持不变
│   ├── Cargo.toml
│   └── src/
└── src/                # 原 forge-core 的所有代码将移动到这里
    ├── lib.rs          # 核心运行时代码 + 再导出宏
    ├── client.rs
    ├── openapi_to_mcp.rs
    └── ... (所有原 forge-core 的模块)
```

### 3.3. `Cargo.toml` 实施细节

**`service_kit/service-kit-macros/Cargo.toml`**:
-   这个文件基本保持不变，但它对 `forge-core` 的依赖需要被移除，因为它所需要的数据结构现在将由 `service_kit` crate 直接提供。为了避免循环依赖，我们需要将 `ApiMetadata` 等核心数据结构的定义移动到一个新的、更底层的 crate (例如 `service-kit-types`)，或者在宏中通过 `::service_kit::...` 路径来引用。为简化起见，我们将选择后者。宏 crate 对 `forge-core` 的依赖将被改为对 `service_kit` 的依赖。

**`service_kit/Cargo.toml`**:
-   将 `forge-core/Cargo.toml` 中的所有 `[dependencies]` 合并进来。
-   移除对 `forge-core` 的 `path` 依赖。
-   保留对 `service-kit-macros` 的依赖：`service-kit-macros = { path = "service-kit-macros" }`。
-   可以考虑添加一个 `macros` feature，以便用户可以选择性地禁用宏。

```toml
# service_kit/Cargo.toml (示例)
[package]
name = "service_kit"
# ...

[dependencies]
# 原 forge-core 的依赖
axum = { version = "0.8", features = ["json"] }
inventory = "0.3"
rmcp = { version = "0.5.0" }
utoipa = { version = "5.4.0" }
# ... 等等

# 依赖自己的过程宏 crate
service-kit-macros = { path = "service-kit-macros", optional = true }

[features]
default = ["macros"]
macros = ["dep:service-kit-macros"]
```

### 3.4. 代码实施细节

**`service_kit/src/lib.rs` (新的门面)**:
-   将 `forge-core/src/lib.rs` 的全部内容移动到这里。
-   在文件的顶部，添加宏的再导出语句：

```rust
// service_kit/src/lib.rs

// 再导出过程宏，当 "macros" feature 启用时
#[cfg(feature = "macros")]
pub use service-kit-macros::{api, api_dto};

// ... 此处是原 forge-core/src/lib.rs 的所有内容 ...
// ... 包括所有模块声明 (pub mod client; pub mod openapi_to_mcp; 等)
// ... 和所有数据结构定义 (pub struct ApiMetadata { ... })
```

**`service_kit/service-kit-macros/src/lib.rs`**:
-   宏代码中所有对 `::forge_core::...` 的引用都需要被修改为 `::service_kit::...`。

```rust
// service-kit-macros/src/lib.rs (示例)

// ...
params_tokens.push(quote! {
    ::service_kit::ApiParameter { // 原为 ::forge_core::ApiParameter
        name: #param_name,
        param_in: ::service_kit::ParamIn::Path, // 原为 ::forge_core::ParamIn::Path
        // ...
    }
});
// ...
```

## 4. 实施路线图

1.  **Phase 1: 文件和目录结构调整**
    -   [ ] 将 `forge-core/src/` 下的所有文件移动到 `service_kit/src/`。
    -   [ ] 删除 `forge-core` 目录。
    -   [ ] 将 `forge-core/Cargo.toml` 的依赖合并到 `service_kit/Cargo.toml`。

2.  **Phase 2: 代码和路径修复**
    -   [ ] 更新 `service_kit/src/lib.rs`，加入 `pub use` 语句来再导出宏。
    -   [ ] 修改 `service-kit-macros` crate，将其对 `forge-core` 的依赖改为 `service_kit`，并更新所有代码中的路径引用。

3.  **Phase 3: `service-template` 更新与验证**
    -   [ ] 更新 `service-template/Cargo.toml`，使其只依赖 `service_kit` 一个 crate。
    -   [ ] 更新 `service-template` 的代码，将所有 `use forge_core::...` 和 `use service-kit-macros::...` 的地方都改为 `use service_kit::...`。
    -   [ ] 运行 `cargo generate` 和 `cargo check` 验证新生成的项目可以正常编译。

## 5. 最终优势

-   **简化用户体验**: 用户只需 `[dependencies] service_kit = "..."`。
-   **内聚性**: `service_kit` 成为一个单一的、高内聚的功能单元。
-   **易于发布**: 管理和发布单个 crate 比管理多个独立的 crate 简单得多。
-   **清晰的边界**: 内部实现（如 `service-kit-macros`）被隐藏，API 边界清晰。

