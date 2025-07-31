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

// A struct to parse the macro's attributes, e.g., `#[ApiDto(rename_all = "snake_case")]`
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

#[proc_macro_attribute]
pub fn ApiDto(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ApiDtoArgs);
    let mut input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;
    
    let rename_all_strategy = args.rename_all.unwrap_or_else(|| "camelCase".to_string());
    
    // --- Get TS output dir from Cargo.toml or use default ---
    let ts_output_dir = get_ts_output_dir().unwrap_or_else(|| "generated/ts/".to_string());

    // Consolidate all attributes to be added
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

    let attr_tokens: proc_macro2::TokenStream = attributes_to_add.into();
    let parsed_attrs: Vec<syn::Attribute> =
        syn::parse::Parser::parse(syn::Attribute::parse_outer, attr_tokens.into())
            .expect("Failed to parse attributes");
    input.attrs.extend(parsed_attrs);


    // --- Handle recursive schema for utoipa ---
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

/// Reads CARGO_MANIFEST_DIR, parses Cargo.toml, and gets the ts_output_dir from metadata.
fn get_ts_output_dir() -> Option<String> {
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


// Helper functions (is_recursive_type, etc.) remain the same
fn is_recursive_type(path: &syn::Path, self_name: &str) -> bool {
    if let Some(segment) = path.segments.last() {
        let type_name = segment.ident.to_string();
        if type_name == "Box" || type_name == "Option" {
             if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(Type::Path(inner_type_path))) = args.args.first() {
                    if type_name == "Option" {
                         if let Some(inner_segment) = inner_type_path.path.segments.last() {
                             if inner_segment.ident == "Box" {
                                 return is_recursive_boxed_type(&inner_segment, self_name);
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
