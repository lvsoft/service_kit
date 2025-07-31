//! # Service Kit - A Proc-Macro for Streamlined Microservice Development
//!
//! `service_kit` provides a procedural attribute macro `#[api_dto]` designed to
//! reduce boilerplate and enforce best practices when creating Data Transfer Objects (DTOs)
//! in Rust-based microservices.

use proc_macro::TokenStream;
use quote::quote;
use std::env;
use std::fs;
use std::path::PathBuf;
use syn::{
    parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Expr, Fields,
    GenericArgument, Lit, Meta, PathArguments, PathSegment, Type,
};
use toml::Value;

/// A helper struct to parse the arguments passed to the `api_dto` macro.
///
/// Currently, it only supports `rename_all = "..."`.
///
/// # Example
///
/// ```ignore
/// #[api_dto(rename_all = "snake_case")]
/// ```
#[derive(Debug, Default)]
struct ApiDtoArgs {
    rename_all: Option<String>,
}

impl syn::parse::Parse for ApiDtoArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ApiDtoArgs::default();
        let meta_list = Punctuated::<Meta, syn::Token![,]>::parse_terminated(input)?;

        for meta in meta_list {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename_all") {
                    if let Expr::Lit(expr_lit) = nv.value {
                        if let Lit::Str(lit_str) = expr_lit.lit {
                             args.rename_all = Some(lit_str.value());
                        }
                    }
                }
            }
        }
        Ok(args)
    }
}

/// A procedural attribute macro to derive essential traits and apply conventions
/// for API Data Transfer Objects (DTOs).
///
/// This macro automates the implementation of common traits and standards, allowing
/// developers to focus on defining the data structure.
///
/// # Injected Traits and Attributes:
///
/// - `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, ts_rs::TS)]`
/// - `#[serde(rename_all = "...")]`: Defaults to `"camelCase"`, but can be overridden.
/// - `#[ts(export, export_to = "...")]`: The output path defaults to `"generated/ts/"` or
///   can be configured globally in `Cargo.toml`.
///
/// # Special Handling:
///
/// - **Recursive Structs**: Automatically injects `#[schema(value_type = Object)]` for
///   fields that are self-referential (e.g., `Option<Box<Self>>`), preventing `utoipa`
///   from failing compilation due to infinite recursion.
///
/// # Configuration:
///
/// The macro can be customized in two ways:
///
/// 1.  **Macro Arguments**: Override the JSON naming convention.
///     ```ignore
///     #[api_dto(rename_all = "snake_case")]
///     pub struct MyDto { /* ... */ }
///     ```
///
/// 2.  **Global `Cargo.toml` Metadata**: Configure the default output directory for TypeScript types.
///     ```toml
///     # In your service's Cargo.toml
///     [package.metadata.service_kit]
///     ts_output_dir = "frontend/src/generated/types/"
///     ```
#[proc_macro_attribute]
pub fn api_dto(attr: TokenStream, item: TokenStream) -> TokenStream {
    // ... (implementation remains the same)
    let args = parse_macro_input!(attr as ApiDtoArgs);
    let mut input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;
    
    let rename_all_strategy = args.rename_all.unwrap_or_else(|| "camelCase".to_string());
    
    let ts_output_dir = get_ts_output_dir().unwrap_or_else(|| "generated/ts/".to_string());

    let attributes_to_add = quote! {
        #[derive(
            Debug,
            Clone,
            serde::Serialize,
            serde::Deserialize,
            utoipa::ToSchema,
            ts_rs::TS
        )]
        #[serde(rename_all = #rename_all_strategy)]
        #[ts(export, export_to = #ts_output_dir)]
    };

    let parsed_attrs: Vec<syn::Attribute> =
        syn::parse::Parser::parse(syn::Attribute::parse_outer, attributes_to_add.into())
            .expect("Failed to parse attributes");
    input.attrs.extend(parsed_attrs);

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            for field in fields.named.iter_mut() {
                if let Type::Path(type_path) = &field.ty {
                    if is_recursive_type(&type_path.path, struct_name.to_string().as_str()) {
                        field.attrs.push(syn::parse_quote! {
                            #[schema(value_type = Object)]
                        });
                    }
                }
            }
        }
    }

    let output = quote! {
        #input
    };

    output.into()
}

/// Reads the `CARGO_MANIFEST_DIR` environment variable to locate the calling crate's
/// `Cargo.toml`, parses it, and extracts the `ts_output_dir` from the
/// `[package.metadata.service_kit]` table.
fn get_ts_output_dir() -> Option<String> {
    // ... (implementation remains the same)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let cargo_toml_path = PathBuf::from(manifest_dir).join("Cargo.toml");
    
    let toml_content = fs::read_to_string(cargo_toml_path).ok()?;
    let toml_value: Value = toml::from_str(&toml_content).ok()?;

    let output_dir = toml_value
        .get("package")?
        .get("metadata")?
        .get("service_kit")?
        .get("ts_output_dir")?
        .as_str()?;
        
    Some(output_dir.to_string())
}

/// Checks if a field's type is a recursive reference to its own struct,
/// specifically looking for `Box<Self>` or `Option<Box<Self>>`.
fn is_recursive_type(path: &syn::Path, self_name: &str) -> bool {
    // ... (implementation remains the same)
    if let Some(segment) = path.segments.last() {
        let type_name = segment.ident.to_string();
        if type_name == "Box" || type_name == "Option" {
             if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(Type::Path(inner_type_path))) = args.args.first() {
                    if type_name == "Option" {
                         if let Some(inner_segment) = inner_type_path.path.segments.last() {
                             if inner_segment.ident == "Box" {
                                 return is_recursive_boxed_type(inner_segment, self_name);
                             }
                         }
                    } else {
                        return is_recursive_boxed_type(segment, self_name);
                    }
                }
             }
        }
    }
    false
}

/// A helper for `is_recursive_type` that checks if a `PathSegment`'s generic
/// argument is a `Box` pointing to the struct `self_name`.
fn is_recursive_boxed_type(segment: &PathSegment, self_name: &str) -> bool {
    // ... (implementation remains the same)
     if let PathArguments::AngleBracketed(args) = &segment.arguments {
         if let Some(GenericArgument::Type(Type::Path(inner_type))) = args.args.first() {
             if let Some(inner_segment) = inner_type.path.segments.last() {
                 return inner_segment.ident == self_name;
             }
         }
     }
     false
}
