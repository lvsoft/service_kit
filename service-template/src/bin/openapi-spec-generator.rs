use {{crate_name}}::ApiDoc;
use utoipa::OpenApi;

// This binary's sole purpose is to print the OpenAPI JSON specification
// to standard output. It is invoked by `cargo forge api-cli`.
fn main() {
    let spec = ApiDoc::openapi().to_pretty_json().unwrap();
    print!("{}", spec);
}
