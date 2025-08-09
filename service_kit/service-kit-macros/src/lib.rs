extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2;
use quote::{quote};
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Attribute, Ident, ItemFn, LitStr,
    Result, Token, Type, Pat, punctuated::Punctuated
};

// ... (ApiMacroArgs and other helpers remain the same)
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

    let fn_name = &item.sig.ident;
    let operation_id = fn_name.to_string();
    
    // --- Handler Logic Generation ---
    let (param_declarations, call_args) = generate_param_extraction_logic(&item);

    // This is the real handler logic that will be placed inside the Arc.
    let handler_logic = quote! {
        // This async block is the core of the type-erased handler.
        async move {
            // The `params` variable is the `&serde_json::Value` passed to the closure.
            #param_declarations

            // Call the original function with the extracted parameters.
            let result = #fn_name(#(#call_args),*).await;

            // Convert the result into an Axum Response.
            // This assumes the function returns something that implements `IntoResponse`.
            Ok(result.into_response())
        }
    };

    // --- The rest of the macro (utoipa, ctor) ---
    // ... (utoipa_path_gen logic remains the same)
    let http_method_str = args_parsed.method.to_string().to_lowercase();
    let http_method = Ident::new(&http_method_str, args_parsed.method.span());
    let path_str = args_parsed.path;
    let (summary, _description) = parse_doc_comments(&item.attrs);
    let mut utoipa_params = Vec::new();
    for arg in &item.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                 let param_type = &pat_type.ty;
                if let Some(inner_type) = get_inner_type(param_type, "Query") {
                    utoipa_params.push(quote! { ( #param_name = inline(#inner_type), Query) });
                } else if let Some(inner_type) = get_inner_type(param_type, "Path") {
                    utoipa_params.push(quote! { (#param_name = #inner_type, Path, description = "ID") });
                }
            } else if let Pat::TupleStruct(pat_tuple) = &*pat_type.pat {
                 if pat_tuple.path.is_ident("Path") {
                    if let Some(Pat::Ident(inner_pat)) = pat_tuple.elems.first() {
                        let param_name = inner_pat.ident.to_string();
                        let param_type = &pat_type.ty;
                         if let Some(inner_type) = get_inner_type(param_type, "Path") {
                            utoipa_params.push(quote! { (#param_name = #inner_type, Path, description = "ID") });
                        }
                    }
                }
            }
        }
    }
    let params_tokens = if utoipa_params.is_empty() {
        quote! {}
    } else {
        quote! { params( #(#utoipa_params),* ), }
    };
    let (status_code, response_body) = if let syn::ReturnType::Type(_, ty) = &item.sig.output {
        if let Some(inner_type) = get_inner_type_from_impl_trait(ty, "IntoResponse") {
             if let Some(json_inner) = get_inner_type(inner_type, "Json") {
                 (quote! { 200 }, quote! { body = #json_inner })
             } else {
                 (quote! { 200 }, quote! { body = String, description = "Generic response" })
             }
        } else if let Some(inner_type) = get_inner_type(ty, "Json") {
            (quote! { 200 }, quote! { body = #inner_type })
        } else {
            (quote! { 200 }, quote! { body = String, description = "Plain text response" })
        }
    } else {
        (quote! { 204 }, quote! { description = "No Content" })
    };
    let utoipa_path_gen = quote! {
        #[utoipa::path(
            #http_method,
            path = #path_str,
            operation_id = #operation_id,
            tag = "App",
            #params_tokens
            responses(
                (status = #status_code, description = #summary, #response_body)
            ),
        )]
    };
    let ctor_fn_name = Ident::new(
        &format!("__register_{}", fn_name.to_string()),
        fn_name.span(),
    );

    let output = quote! {
        #utoipa_path_gen
        #item

        #[::ctor::ctor]
        fn #ctor_fn_name() {
            let handler = ::forge_core::handler::ApiMethodHandler {
                operation_id: #operation_id,
                handler: std::sync::Arc::new(|params: &::serde_json::Value| Box::pin(#handler_logic)),
            };
            ::forge_core::handler::register_handler(handler);
        }
    };

    output.into()
}

/// Generates the token stream for extracting function parameters from a `serde_json::Value`.
fn generate_param_extraction_logic(item: &ItemFn) -> (proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>) {
    let mut declarations = Vec::new();
    let mut call_args = Vec::new();

    for arg in &item.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            let param_pat = &pat_type.pat;
            let param_type = &pat_type.ty;

            let (declaration, call_arg) =
                if let Some(_inner_type) = get_inner_type(param_type, "Path") {
                    if let Pat::TupleStruct(pat_tuple) = &**param_pat {
                        if pat_tuple.path.is_ident("Path") {
                             if let Some(Pat::Ident(inner_pat)) = pat_tuple.elems.first() {
                                let param_name = &inner_pat.ident;
                                let param_name_str = param_name.to_string();
                                (
                                    quote! {
                                        let #param_name = serde_json::from_value(
                                            params.get(#param_name_str)
                                                .cloned()
                                                .unwrap_or(serde_json::Value::Null)
                                        ).expect("Failed to deserialize Path parameter");
                                    },
                                    quote! { axum::extract::Path(#param_name) },
                                )
                            } else {
                                (quote!{}, quote!{unimplemented!("Unsupported Path pattern")})
                            }
                        } else {
                            (quote!{}, quote!{unimplemented!("Unsupported tuple struct pattern")})
                        }
                    } else {
                       (quote!{}, quote!{unimplemented!("Path must be a tuple struct pattern like Path(id)")})
                    }
                } else if let Some(_inner_type) = get_inner_type(param_type, "Query") {
                    let temp_query_var = quote::format_ident!("__query_params");
                    (
                        quote! {
                            let #temp_query_var = serde_json::from_value(params.clone())
                                .expect("Failed to deserialize Query parameters");
                        },
                        quote! { axum::extract::Query(#temp_query_var) },
                    )
                } else {
                    (quote! { /* Unsupported parameter type */ }, quote! { unimplemented!() })
                };

            declarations.push(declaration);
            call_args.push(call_arg);
        }
    }
    (quote! { #(#declarations)* }, call_args)
}

// ... (parse_doc_comments, get_inner_type, etc. remain the same)
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

fn get_inner_type<'a>(ty: &'a Type, type_name: &str) -> Option<&'a Type> {
    if let Type::Path(type_path) = ty {
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

fn get_inner_type_from_impl_trait<'a>(
    ty: &'a Type,
    trait_name: &str,
) -> Option<&'a Type> {
    if let syn::Type::ImplTrait(type_impl_trait) = ty {
        if let Some(syn::TypeParamBound::Trait(trait_bound)) = type_impl_trait.bounds.first() {
            if let Some(segment) = trait_bound.path.segments.last() {
                if segment.ident == trait_name {
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

// --- `api_dto` and its helpers ---

#[derive(Debug, Default)]
struct ApiDtoArgs {
    rename_all: Option<String>,
}

impl syn::parse::Parse for ApiDtoArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ApiDtoArgs::default();
        let meta_list = Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input)?;

        for meta in meta_list {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename_all") {
                    if let syn::Expr::Lit(expr_lit) = nv.value {
                        if let syn::Lit::Str(lit_str) = expr_lit.lit {
                            args.rename_all = Some(lit_str.value());
                        }
                    }
                }
            }
        }
        Ok(args)
    }
}

#[proc_macro_attribute]
pub fn api_dto(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ApiDtoArgs);
    let mut input = parse_macro_input!(item as syn::DeriveInput);
    
    let rename_all_strategy = args.rename_all.unwrap_or_else(|| "camelCase".to_string());

    let attributes_to_add = quote! {
        #[derive(
            Debug,
            Clone,
            serde::Serialize,
            serde::Deserialize,
            utoipa::ToSchema
        )]
        #[serde(rename_all = #rename_all_strategy)]
    };

    let parsed_attrs: Vec<syn::Attribute> =
        syn::parse::Parser::parse(syn::Attribute::parse_outer, attributes_to_add.into())
            .expect("Failed to parse attributes");
    input.attrs.extend(parsed_attrs);

    if let syn::Data::Struct(ref mut data_struct) = input.data {
        if let syn::Fields::Named(ref mut fields) = data_struct.fields {
            for field in fields.named.iter_mut() {
                if let Type::Path(type_path) = &field.ty {
                    if is_recursive_type(&type_path.path, &input.ident.to_string()) {
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

fn is_recursive_type(path: &syn::Path, self_name: &str) -> bool {
    if let Some(segment) = path.segments.last() {
        let type_name = segment.ident.to_string();
        if type_name == "Box" || type_name == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(Type::Path(inner_type_path))) = args.args.first()
                {
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

fn is_recursive_boxed_type(segment: &syn::PathSegment, self_name: &str) -> bool {
    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(syn::GenericArgument::Type(Type::Path(inner_type))) = args.args.first() {
            if let Some(inner_segment) = inner_type.path.segments.last() {
                return inner_segment.ident == self_name;
            }
        }
    }
    false
}