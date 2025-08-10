extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Attribute, FnArg, Ident, ItemFn, LitStr,
    Pat, Result, ReturnType, Token, Type, punctuated::Punctuated
};


// --- Macro Implementations ---

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
    let item_fn = parse_macro_input!(input as ItemFn);
    let args_parsed = parse_macro_input!(args as ApiMacroArgs);

    let fn_ident = &item_fn.sig.ident;
    let fn_name_str = fn_ident.to_string();
    let method_str = args_parsed.method.to_string();
    let path_str = args_parsed.path.value();
    let (summary, description) = parse_doc_comments(&item_fn.attrs);

    // --- Parse Parameters and Request Body ---
    let mut params_tokens = Vec::new();
    let mut request_body_token = quote! { None };

    // For building runtime wrapper
    let mut arg_prepare_tokens = Vec::new();
    let mut call_args_tokens = Vec::new();

    for arg in &item_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Some(inner_type) = get_inner_type(&pat_type.ty, "Path") {
                if let Pat::TupleStruct(pat_tuple) = &*pat_type.pat {
                     if let Some(Pat::Ident(inner_pat)) = pat_tuple.elems.first() {
                        let param_name = inner_pat.ident.to_string();
                        let type_name = type_to_string(inner_type);
                        params_tokens.push(quote! {
                            ::service_kit::ApiParameter {
                                name: #param_name,
                                param_in: ::service_kit::ParamIn::Path,
                                description: "", // TODO: Parse from attributes
                                required: true,
                                type_name: #type_name,
                            }
                        });
                        // runtime wrapper: read string and wrap
                        let var_ident = &inner_pat.ident;
                        arg_prepare_tokens.push(quote! {
                            let #var_ident: String = match params.get(#param_name).and_then(|v| v.as_str()) {
                                Some(s) => s.to_string(),
                                None => String::new(),
                            };
                            let #var_ident = axum::extract::Path::<String>(#var_ident);
                        });
                        call_args_tokens.push(quote! { #var_ident });
                    }
                } else if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    // Also support `id: Path<String>` style
                    let param_name = pat_ident.ident.to_string();
                    let type_name = type_to_string(inner_type);
                    params_tokens.push(quote! {
                        ::service_kit::ApiParameter {
                            name: #param_name,
                            param_in: ::service_kit::ParamIn::Path,
                            description: "",
                            required: true,
                            type_name: #type_name,
                        }
                    });
                    let var_ident = &pat_ident.ident;
                    arg_prepare_tokens.push(quote! {
                        let #var_ident: String = match params.get(#param_name).and_then(|v| v.as_str()) {
                            Some(s) => s.to_string(),
                            None => String::new(),
                        };
                        let #var_ident = axum::extract::Path::<String>(#var_ident);
                    });
                    call_args_tokens.push(quote! { #var_ident });
                }
            } else if let Some(inner_type) = get_inner_type(&pat_type.ty, "Query") {
                // Support both `Query(params): Query<T>` and `params: Query<T>` patterns
                let param_name_opt = if let Pat::TupleStruct(pat_tuple) = &*pat_type.pat {
                    pat_tuple
                        .elems
                        .first()
                        .and_then(|p| match p { Pat::Ident(pi) => Some(pi.ident.to_string()), _ => None })
                } else if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some(pat_ident.ident.to_string())
                } else { None };

                if let Some(param_name) = param_name_opt {
                    let type_name = type_to_string(inner_type);
                    params_tokens.push(quote! {
                        ::service_kit::ApiParameter {
                            name: #param_name,
                            param_in: ::service_kit::ParamIn::Query,
                            description: "", // TODO: Parse from attributes
                            required: true, // TODO: Detect Option
                            type_name: #type_name,
                        }
                    });
                    // runtime wrapper: deserialize whole params into T
                    let var_ident = format_ident!("{}", param_name);
                     let inner_ty_tokens = quote! { #inner_type };
                    arg_prepare_tokens.push(quote! {
                        let #var_ident: #inner_ty_tokens = match serde_json::from_value(params.clone()) {
                            Ok(v) => v,
                             Err(e) => return Err(::service_kit::error::Error::SerdeJson(e)),
                        };
                        let #var_ident = axum::extract::Query::<#inner_ty_tokens>(#var_ident);
                    });
                    call_args_tokens.push(quote! { #var_ident });
                }
            } else if let Some(inner_type) = get_inner_type(&pat_type.ty, "Json") {
                let type_name = type_to_string(inner_type);
                request_body_token = quote! {
                    Some(&::service_kit::ApiRequestBody {
                        description: "", // TODO: Parse from attributes
                        required: true,
                        type_name: #type_name,
                    })
                };
                // runtime wrapper: deserialize whole params into body T
                 let inner_ty_tokens = quote! { #inner_type };
                 let json_ident = syn::Ident::new("__json_body", proc_macro2::Span::call_site());
                arg_prepare_tokens.push(quote! {
                    let #json_ident: #inner_ty_tokens = match serde_json::from_value(params.clone()) {
                         Ok(v) => v,
                         Err(e) => return Err(::service_kit::error::Error::SerdeJson(e)),
                    };
                    let #json_ident = axum::Json::<#inner_ty_tokens>(#json_ident);
                });
                call_args_tokens.push(quote! { #json_ident });
            }
        }
    }

    // --- Parse Responses ---
    let mut responses_tokens = Vec::new();
    if let ReturnType::Type(_, ty) = &item_fn.sig.output {
        if let Some(inner_type) = get_inner_type(ty, "Json") {
            let type_name = type_to_string(inner_type);
            responses_tokens.push(quote! {
                ::service_kit::ApiResponse {
                    status_code: 200,
                    description: #summary,
                    type_name: Some(#type_name),
                }
            });
        }
    }
    // Add a default response if none was parsed
    if responses_tokens.is_empty() {
        responses_tokens.push(quote! {
            ::service_kit::ApiResponse { status_code: 200, description: "Success", type_name: None }
        });
    }

    // --- Generate Static Metadata ---
    let params_ident = format_ident!("__API_PARAMS_{}", fn_name_str.to_uppercase());
    let responses_ident = format_ident!("__API_RESPONSES_{}", fn_name_str.to_uppercase());
    let request_body_ident = format_ident!("__API_REQ_BODY_{}", fn_name_str.to_uppercase());

    let exec_fn_ident = format_ident!("__API_EXEC_{}", fn_name_str.to_uppercase());

    let static_metadata = quote! {
        #[allow(non_upper_case_globals)]
        const #params_ident: &[::service_kit::ApiParameter] = &[#(#params_tokens),*];
        #[allow(non_upper_case_globals)]
        const #responses_ident: &[::service_kit::ApiResponse] = &[#(#responses_tokens),*];
        #[allow(non_upper_case_globals)]
        const #request_body_ident: Option<&'static ::service_kit::ApiRequestBody> = #request_body_token;

        ::service_kit::inventory::submit! {
            ::service_kit::ApiMetadata {
                operation_id: #fn_name_str,
                method: #method_str,
                path: #path_str,
                summary: #summary,
                description: #description,
                parameters: #params_ident,
                request_body: #request_body_ident,
                responses: #responses_ident,
            }
        }

        // Static handler function for REST/MCP routers
        fn #exec_fn_ident(__params_ref: &serde_json::Value) -> ::service_kit::handler::DynHandlerFuture {
            let __params_json = __params_ref.clone();
            Box::pin(async move {
                let params = __params_json.clone();
                #(#arg_prepare_tokens)*
                let __resp = #fn_ident(#(#call_args_tokens),*).await;
                let __resp = ::axum::response::IntoResponse::into_response(__resp);
                Ok(__resp)
            })
        }

        // Register executable handler
        ::service_kit::inventory::submit! {
            ::service_kit::handler::ApiHandlerInventory {
                operation_id: #fn_name_str,
                handler: #exec_fn_ident,
            }
        }
    };

    // --- Final Output ---
    let output = quote! {
        #static_metadata
        #item_fn
    };

    output.into()
}

fn type_to_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
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
    
    let type_name = input.ident.clone();
    let type_name_str = type_name.to_string();

    let rename_all_strategy = args.rename_all.unwrap_or_else(|| "camelCase".to_string());

    let attributes_to_add = quote! {
        #[derive(
            Debug,
            Clone,
            serde::Serialize,
            serde::Deserialize,
            ::service_kit::utoipa::ToSchema
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

    // 注册 DTO schema 到 inventory
    let registration = quote! {
        ::service_kit::inventory::submit! {
            ::service_kit::ApiDtoMetadata {
                name: #type_name_str,
                schema_provider: || {
                    (
                        #type_name_str.to_string(),
                        <#type_name as ::service_kit::utoipa::PartialSchema>::schema(),
                    )
                },
            }
        }
    };

    let output = quote! {
        #input
        #registration
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