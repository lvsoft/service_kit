use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, AngleBracketedGenericArguments, Data, DeriveInput, Expr, Fields, FnArg,
    GenericArgument, Ident, ItemFn, Lit, Meta, PatType, PathArguments, PathSegment, Result,
    ReturnType, Token, Type, TypePath, TypeTuple,
    punctuated::Punctuated,
};

// --- Argument Parsing for `api_route` ---
struct ApiRouteArgs {
    method: Ident,
    path: Lit,
    other_meta: Punctuated<Meta, Token![,]>,
}

impl Parse for ApiRouteArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let method: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let path: Lit = input.parse()?;

        let other_meta = if input.is_empty() {
            Punctuated::new()
        } else {
            input.parse::<Token![,]>()?;
            Punctuated::parse_terminated(input)?
        };

        Ok(ApiRouteArgs {
            method,
            path,
            other_meta,
        })
    }
}

#[proc_macro_attribute]
pub fn api_route(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_args = parse_macro_input!(args as ApiRouteArgs);
    let item = parse_macro_input!(input as ItemFn);

    let method_ident = parsed_args.method;
    let path = parsed_args.path;
    let other_args = parsed_args.other_meta;

    // --- Inferred arguments ---
    let mut inferred_args = Vec::new();

    // Response Inference
    if let ReturnType::Type(_, ty) = &item.sig.output {
        if let Some(response_type) = get_json_response_type(ty) {
            inferred_args.push(quote! {
                responses(
                    (status = 200, description = "Successful response", body = #response_type)
                )
            });
        }
    }

    // Query Param Inference
    if let Some(query_param_type) = get_query_param_type(&item.sig.inputs) {
        inferred_args.push(quote! {
            params(#query_param_type)
        });
    }

    // --- Combine all arguments ---
    let utoipa_args = quote! {
        #method_ident,
        path = #path,
        #(#inferred_args,)*
        #other_args
    };

    let item_fn = &item;

    TokenStream::from(quote! {
        #[utoipa::path(
            #utoipa_args
        )]
        #item_fn
    })
}


/// Extracts the inner type `T` from a `Query<T>` in function arguments.
fn get_query_param_type(inputs: &Punctuated<FnArg, Token![,]>) -> Option<&Type> {
    for arg in inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Type::Path(TypePath { path, .. }) = &**ty {
                if let Some(segment) = path.segments.last() {
                    if segment.ident == "Query" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                                return Some(inner_ty);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts the inner type `T` from `Json<T>`, `(StatusCode, Json<T>)`, etc., in return types.
fn get_json_response_type(return_type: &Type) -> Option<proc_macro2::TokenStream> {
    match return_type {
        Type::Path(TypePath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                if segment.ident == "Json" {
                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
                        &segment.arguments
                    {
                        if let Some(GenericArgument::Type(inner_ty)) = args.first() {
                            return Some(quote! { #inner_ty });
                        }
                    }
                }
            }
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            for elem in elems {
                if let Some(json_type) = get_json_response_type(elem) {
                    return Some(json_type);
                }
            }
        }
        _ => (),
    }
    None
}

// --- `api_dto` / `api_params` and their helpers ---

#[derive(Debug, Default)]
struct ApiDtoArgs {
    rename_all: Option<String>,
}

impl syn::parse::Parse for ApiDtoArgs {
    fn parse(input: ParseStream) -> Result<Self> {
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

#[proc_macro_attribute]
pub fn api_dto(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ApiDtoArgs);
    let mut input = parse_macro_input!(item as DeriveInput);
    
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

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
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

#[proc_macro_attribute]
pub fn api_params(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ApiDtoArgs);
    let mut input = parse_macro_input!(item as DeriveInput);
    
    let rename_all_strategy = args.rename_all.unwrap_or_else(|| "camelCase".to_string());

    let attributes_to_add = quote! {
        #[derive(
            Debug,
            Clone,
            serde::Deserialize,
            utoipa::ToSchema,
            utoipa::IntoParams
        )]
        #[serde(rename_all = #rename_all_strategy)]
    };

    let parsed_attrs: Vec<syn::Attribute> =
        syn::parse::Parser::parse(syn::Attribute::parse_outer, attributes_to_add.into())
            .expect("Failed to parse attributes");
    input.attrs.extend(parsed_attrs);

    // Note: Recursive type check is omitted for params as it's less common.
    // Can be added if needed.

    let output = quote! {
        #input
    };

    output.into()
}


fn is_recursive_type(path: &syn::Path, self_name: &str) -> bool {
    if let Some(segment) = path.segments.last() {
        let type_name = segment.ident.to_string();
        if type_name == "Box" || type_name == "Option" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(Type::Path(inner_type_path))) = args.args.first()
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

fn is_recursive_boxed_type(segment: &PathSegment, self_name: &str) -> bool {
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(GenericArgument::Type(Type::Path(inner_type))) = args.args.first() {
            if let Some(inner_segment) = inner_type.path.segments.last() {
                return inner_segment.ident == self_name;
            }
        }
    }
    false
}
