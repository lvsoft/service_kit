extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, Attribute, Ident, ItemFn, LitStr, Token, Result};
use heck::ToSnakeCase;

/// A procedural macro to automatically expose a function as a REST and MCP API endpoint.
///
/// This macro performs several key tasks:
/// 1.  Generates a `#[utoipa::path]` attribute to create an OpenAPI specification for the function.
/// 2.  Generates a `#[ctor]` function that runs at program startup to register the API handler.
// --- 1. Define a struct for parsing macro arguments ---
struct ApiMacroArgs {
    method: Ident,
    path: LitStr,
}

impl Parse for ApiMacroArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let method: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let path: LitStr = input.parse()?;
        Ok(ApiMacroArgs { method, path })
    }
}


#[proc_macro_attribute]
pub fn api(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemFn);
    let args_parsed = parse_macro_input!(args as ApiMacroArgs);
    
    // --- 2. Extract Parsed Arguments ---
    let http_method_str = args_parsed.method.to_string().to_lowercase();
    let http_method = Ident::new(&http_method_str, args_parsed.method.span());
    let path_str = args_parsed.path;

    // --- 3. Parse Function Details ---
    let fn_name = &item.sig.ident;
    let operation_id = fn_name.to_string();
    let (summary, _description) = parse_doc_comments(&item.attrs);
    
    // --- 4. Generate `utoipa::path` ---
    
    // Parse parameters
    let mut utoipa_params = Vec::new();
    for arg in &item.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            let param_name = pat_type.pat.to_token_stream().to_string().to_snake_case();
            let param_type = &pat_type.ty;
            // This is a simplified example. A real implementation needs to handle
            // different extractors like Path, Query, Json, etc. and their specific syntaxes.
            if let Some(inner_type) = get_inner_type(param_type, "Query") {
                 utoipa_params.push(quote! { ( #param_name = inline(#inner_type), Query) });
            } else if let Some(inner_type) = get_inner_type(param_type, "Path") {
                // Corrected utoipa syntax for path parameters
                 utoipa_params.push(quote! { (#param_name = #inner_type, Path, description = "ID") });
            }
        }
    }
    let params_tokens = if utoipa_params.is_empty() {
        quote! {}
    } else {
        quote! { params( #(#utoipa_params),* ), }
    };

    // Parse response
    let (status_code, response_body) = if let syn::ReturnType::Type(_, ty) = &item.sig.output {
         if let Some(inner_type) = get_inner_type(ty, "Json") {
             (quote! { 200 }, quote! { body = #inner_type })
         } else if let Some(inner_type) = get_inner_type_from_impl_trait(ty, "IntoResponse") {
            // A bit of a hack: if the return is `impl IntoResponse`, try to find a `Json<T>` inside.
            // A more robust solution would inspect the function body or have stronger conventions.
             if let Some(json_inner) = get_inner_type(inner_type, "Json") {
                 (quote! { 200 }, quote! { body = #json_inner })
             } else {
                 (quote! { 200 }, quote! { body = String, description = "Generic response" })
             }
         }
         else {
             // Default for other types
             (quote! { 200 }, quote! { body = String, description = "Plain text response" })
         }
    } else {
        // No return type -> 204 No Content
        (quote! { 204 }, quote! { description = "No Content" })
    };

    let utoipa_path_gen = quote! {
        #[utoipa::path(
            #http_method,
            path = #path_str,
            operation_id = #operation_id,
            tag = "App", // TODO: Make this configurable
            #params_tokens
            responses(
                (status = #status_code, description = #summary, #response_body)
            ),
        )]
    };

    // --- 5. Generate Registration Logic ---
    let ctor_fn_name = Ident::new(&format!("__register_{}", fn_name.to_string()), fn_name.span());
    let handler_logic = quote! {
        // The handler now returns a Result directly.
        Ok(axum::response::Json(serde_json::json!({ "status": "ok" })).into_response())
    };
    
    // --- 6. Assemble Final Token Stream ---
    let output = quote! {
        #utoipa_path_gen
        #item

        #[::ctor::ctor]
        fn #ctor_fn_name() {
            let handler = ::forge_core::handler::ApiMethodHandler {
                operation_id: #operation_id,
                handler: Box::new(|_params: &::serde_json::Value| Box::pin(async move {
                    // The actual logic will be much more complex, involving deserializing `params`
                    // into the function's argument types.
                    #handler_logic
                })),
            };
            ::forge_core::handler::register_handler(handler);
        }
    };

    output.into()
}

/// Parses doc comments (`///` and `/** ... */`) into a summary and description.
fn parse_doc_comments(attrs: &[Attribute]) -> (String, String) {
    let doc_comments: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let syn::Lit::Str(lit) = &expr_lit.lit {
                           return Some(lit.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect();

    let description = doc_comments.join("\n");
    let summary = description.lines().next().unwrap_or("").to_string();
    (summary, description)
}

/// Extracts the inner type from a generic type like `Query<T>` -> `T`.
fn get_inner_type<'a>(ty: &'a syn::Type, type_name: &str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == type_name {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Extracts the inner type from an `impl Trait<Inner>` construct.
fn get_inner_type_from_impl_trait<'a>(ty: &'a syn::Type, trait_name: &str) -> Option<&'a syn::Type> {
    if let syn::Type::ImplTrait(type_impl_trait) = ty {
        if let Some(syn::TypeParamBound::Trait(trait_bound)) = type_impl_trait.bounds.first() {
            if let Some(segment) = trait_bound.path.segments.last() {
                if segment.ident == trait_name {
                    // This is a simplification. It doesn't extract the inner type `T` from `IntoResponse<T>`.
                    // A proper implementation would need to parse the generic arguments of the trait bound.
                    // For now, let's just assume the function body will give us what we need,
                    // and we can try to find a Json<T> inside the arguments.
                     if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return Some(inner);
                        }
                    }
                }
            }
        }
    }
    None
}
