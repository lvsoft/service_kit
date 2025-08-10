use clap::{Arg, Command};
use oas::{OpenAPIV3, PathItem, Referenceable};

pub fn build_cli_from_spec(spec: &OpenAPIV3) -> Command {
    let app = Command::new("forge-api-cli")
        .bin_name("forge-api-cli")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A dynamic OpenAPI CLI client. After providing the URL, use one of the generated subcommands.")
        .arg_required_else_help(true);

    spec.paths.iter().fold(app, |acc, (path, path_item)| {
        add_operations_as_subcommands(acc, path, path_item)
    })
}

fn add_operations_as_subcommands(mut app: Command, path: &str, item: &PathItem) -> Command {
    let command_name_prefix = path
        .trim_start_matches('/')
        .replace('/', ".")
        .replace('{', "")
        .replace('}', "");

    let operations = [
        ("GET", &item.get),
        ("POST", &item.post),
        ("PUT", &item.put),
        ("DELETE", &item.delete),
        ("PATCH", &item.patch),
    ];

    for (method, op_opt) in &operations {
        if let Some(op) = op_opt {
            let command_name = format!("{}.{}", command_name_prefix, method.to_lowercase());
            let static_command_name: &'static str = Box::leak(command_name.into_boxed_str());
            
            let sub_command_about: &'static str = Box::leak(op.summary.as_deref().unwrap_or_else(|| op.description.as_deref().unwrap_or("")).to_owned().into_boxed_str());
            let mut sub_command = Command::new(static_command_name)
                .about(sub_command_about);

            if let Some(params) = &op.parameters {
                for param_ref in params {
                    if let Referenceable::Data(param) = param_ref {
                        let arg_name: &'static str = Box::leak(param.name.clone().into_boxed_str());
                        let arg_help = param.description.as_deref().unwrap_or("").to_owned();

                        let arg = Arg::new(arg_name)
                            .long(arg_name)
                            .help(arg_help)
                            .required(param.required.unwrap_or(false))
                            .action(clap::ArgAction::Set);
                        sub_command = sub_command.arg(arg);
                    }
                }
            }

            if let Some(Referenceable::Data(request_body)) = &op.request_body {
                if request_body.content.contains_key("application/json") {
                    let arg = Arg::new("body")
                        .long("body")
                        .help("The JSON request body as a string.")
                        .required(request_body.required.unwrap_or(false))
                        .action(clap::ArgAction::Set);
                    sub_command = sub_command.arg(arg);
                }
            }
            app = app.subcommand(sub_command);
        }
    }
    app
}


