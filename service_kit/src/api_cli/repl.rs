use crate::api_cli::cli;
use crate::api_cli::completer::ClapCompleter;
use crate::api_cli::error::Result;
use colored::Colorize;
use nu_ansi_term::{Color, Style};
use oas::OpenAPIV3;
use reedline::{
    default_emacs_keybindings, ColumnarMenu, Emacs, KeyCode, KeyModifiers, Reedline, ReedlineEvent,
    ReedlineMenu, Signal, MenuBuilder,
};
use std::borrow::Cow;

struct ReplPrompt;

impl reedline::Prompt for ReplPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Borrowed("forge-api")
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed(">> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: reedline::PromptHistorySearch,
    ) -> Cow<str> {
        Cow::Borrowed("? ")
    }
}

pub async fn start_repl(base_url: &str, spec: &OpenAPIV3) -> Result<()> {
    let command = cli::build_cli_from_spec(spec);
    let completer = ClapCompleter::new(command.clone());

    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_text_style(Style::new().fg(Color::White))
            .with_selected_text_style(Style::new().fg(Color::Black).on(Color::Green))
            .with_description_text_style(Style::new().fg(Color::Yellow)),
    );

    let mut line_editor = Reedline::create()
        .with_completer(Box::new(completer))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    println!("Welcome to the interactive Forge API CLI. Type 'help' for a list of commands, or 'exit' to quit.");

    let prompt = ReplPrompt;

    loop {
        let sig = line_editor.read_line(&prompt)?;

        match sig {
            Signal::Success(buffer) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                if line == "exit" || line == "quit" {
                    break;
                }

                if line == "help" {
                    let _ = command.clone().print_help();
                    continue;
                }

                let mut args = shlex::split(line).unwrap_or_else(|| vec![line.to_string()]);
                // Prepend program name for clap
                args.insert(0, "forge-api-cli".to_string());

                match command.clone().try_get_matches_from(args) {
                    Ok(matches) => {
                        if let Some((subcommand_name, subcommand_matches)) = matches.subcommand() {
                            match crate::api_cli::client::execute_request(
                                base_url,
                                subcommand_name,
                                subcommand_matches,
                                spec,
                            )
                            .await
                            {
                                Ok(_) => (),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
            Signal::CtrlD | Signal::CtrlC => {
                break;
            }
        }
    }

    Ok(())
}
