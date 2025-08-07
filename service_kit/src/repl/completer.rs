use reedline::{Completer, Span, Suggestion};
use clap::{Command};

/// A completer for clap commands.
pub struct ClapCompleter {
    command: Command,
}

impl ClapCompleter {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

fn find_subcommand_suggestions(
    command: &Command,
    parts: &[String],
    current_word: &str,
    span_start: usize,
    span_end: usize,
) -> Vec<Suggestion> {
    let mut current_cmd = command;
    let mut suggestions = Vec::new();

    // Traverse the subcommand path
    let relevant_parts_count = if current_word.is_empty() && !parts.is_empty()
        && parts.last().map_or(false, |p| !p.is_empty()) {
        parts.len()
    } else {
        parts.len().saturating_sub(1)
    };

    for part in parts.iter().take(relevant_parts_count) {
        if let Some(sub_cmd) = current_cmd.get_subcommands().find(|sc| sc.get_name() == part) {
            current_cmd = sub_cmd;
        } else {
            return suggestions;
        }
    }

    // Find matching subcommands
    for sub_cmd in current_cmd.get_subcommands() {
        if sub_cmd.get_name().starts_with(current_word) {
            suggestions.push(Suggestion {
                value: sub_cmd.get_name().to_string(),
                description: sub_cmd.get_about().map(|s| s.to_string()),
                extra: None,
                span: Span::new(span_start, span_end),
                append_whitespace: true,
            });
        }
    }
    suggestions
}

fn find_argument_suggestions(
    command: &Command,
    current_word: &str,
    span_start: usize,
    span_end: usize,
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    if !current_word.starts_with('-') {
        return suggestions;
    }

    for arg in command.get_arguments() {
        if let Some(long) = arg.get_long() {
            let long_flag = format!("--{}", long);
            if long_flag.starts_with(current_word) {
                suggestions.push(Suggestion {
                    value: long_flag,
                    description: arg.get_help().map(|s| s.to_string()),
                    extra: None,
                    span: Span::new(span_start, span_end),
                    append_whitespace: !arg.get_action().takes_values(),
                });
            }
        }
        if let Some(short) = arg.get_short() {
            let short_flag = format!("-{}", short);
            if short_flag.starts_with(current_word) {
                if !suggestions.iter().any(|s| s.value == short_flag) {
                    suggestions.push(Suggestion {
                        value: short_flag,
                        description: arg.get_help().map(|s| s.to_string()),
                        extra: None,
                        span: Span::new(span_start, span_end),
                        append_whitespace: !arg.get_action().takes_values(),
                    });
                }
            }
        }
    }
    suggestions
}

fn find_value_suggestions(
    arg: &clap::Arg,
    current_word: &str,
    span_start: usize,
    span_end: usize,
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    for pv in arg.get_possible_values() {
        if pv.get_name().starts_with(current_word) {
            suggestions.push(Suggestion {
                value: pv.get_name().to_string(),
                description: pv.get_help().map(|s| s.to_string()),
                extra: None,
                span: Span::new(span_start, span_end),
                append_whitespace: true,
            });
        }
    }
    suggestions
}

fn get_command_at_path<'a>(base_cmd: &'a Command, parts: &[String]) -> &'a Command {
    let mut current_cmd = base_cmd;
    for part_name in parts {
        if !part_name.starts_with('-') {
            if let Some(sub_cmd) = current_cmd.get_subcommands().find(|sc| sc.get_name() == part_name) {
                current_cmd = sub_cmd;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    current_cmd
}

impl Completer for ClapCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let line_to_cursor = &line[..pos];

        let parts: Vec<String> = shlex::split(line_to_cursor)
            .unwrap_or_else(|| line_to_cursor.split_whitespace().map(String::from).collect());

        let (current_word, span_start) = if line_to_cursor.ends_with(' ') || parts.is_empty() {
            ("", pos)
        } else {
            let last_part = parts.last().expect("Parts should not be empty");
            (last_part.as_str(), pos - last_part.len())
        };

        let mut current_cmd = &self.command;
        let mut last_arg_opt: Option<&clap::Arg> = None;
        let mut potential_value_completion_context = false;

        // Parse command path
        let effective_parts_for_cmd_structure = &parts[..];

        for (idx, part) in effective_parts_for_cmd_structure.iter().enumerate() {
            if idx == parts.len() - 1 && !line_to_cursor.ends_with(' ') {
                if let Some(last_arg) = last_arg_opt {
                    if last_arg.get_action().takes_values() {
                        potential_value_completion_context = true;
                    }
                }
                break;
            }

            if part.starts_with('-') {
                if let Some(arg_match) = current_cmd.get_arguments().find(|a| {
                    (a.get_long().map_or(false, |l| format!("--{}", l) == *part)) ||
                    (a.get_short().map_or(false, |s| format!("-{}", s) == *part))
                }) {
                    last_arg_opt = Some(arg_match);
                    if !arg_match.get_action().takes_values() {
                        last_arg_opt = None;
                    }
                } else {
                    last_arg_opt = None;
                    break;
                }
            } else {
                if let Some(prev_arg) = last_arg_opt {
                    if prev_arg.get_action().takes_values() {
                        last_arg_opt = None;
                    }
                }

                if let Some(sub_cmd) = current_cmd.get_subcommands().find(|sc| sc.get_name() == part) {
                    current_cmd = sub_cmd;
                    last_arg_opt = None;
                } else {
                    if !last_arg_opt.map_or(false, |arg| arg.get_action().takes_values()) {
                        break;
                    }
                    last_arg_opt = None;
                }
            }
        }

        // Priority 1: Complete argument values
        if potential_value_completion_context {
            if let Some(arg_name_part_idx) = parts.len().checked_sub(2) {
                if let Some(arg_name_part) = parts.get(arg_name_part_idx) {
                    let path_to_arg_command = &parts[..arg_name_part_idx];
                    let command_for_arg = get_command_at_path(&self.command, path_to_arg_command);

                    if let Some(clap_arg) = command_for_arg.get_arguments().find(|a| {
                        a.get_long().map_or(false, |l| format!("--{}", l) == *arg_name_part) ||
                        a.get_short().map_or(false, |s| format!("-{}", s) == *arg_name_part)
                    }) {
                        if clap_arg.get_action().takes_values() {
                            suggestions.extend(find_value_suggestions(clap_arg, current_word, span_start, pos));
                        }
                    }
                }
            }
        }

        // Handle trailing space for value completion
        if line_to_cursor.ends_with(' ') && last_arg_opt.map_or(false, |arg| arg.get_action().takes_values()) {
            if let Some(arg_that_needs_value) = last_arg_opt {
                suggestions.extend(find_value_suggestions(arg_that_needs_value, "", span_start, pos));
            }
        }

        // If no value suggestions, provide other types of completions
        if suggestions.is_empty() {
            let base_cmd_for_suggestions = current_cmd;

            // Argument completion
            if current_word.starts_with('-') {
                suggestions.extend(find_argument_suggestions(base_cmd_for_suggestions, current_word, span_start, pos));
            }

            // Subcommand completion
            suggestions.extend(find_subcommand_suggestions(current_cmd, &[], current_word, span_start, pos));

            // Argument suggestions on trailing space
            if line_to_cursor.ends_with(' ') && current_word.is_empty() {
                let existing_flags: std::collections::HashSet<_> = parts.iter()
                    .filter(|p| p.starts_with("-"))
                    .map(|p| p.as_str())
                    .collect();

                let mut new_arg_suggestions = Vec::new();

                // Long args
                let long_args = find_argument_suggestions(base_cmd_for_suggestions, "--", span_start, pos)
                    .into_iter()
                    .filter(|s| s.value.starts_with("--"));

                for sugg in long_args {
                    if let Some(arg_def) = base_cmd_for_suggestions.get_arguments().find(|a| {
                        a.get_long().map_or(false, |l| format!("--{}", l) == sugg.value)
                    }) {
                        if matches!(*arg_def.get_action(), clap::ArgAction::SetTrue) && existing_flags.contains(sugg.value.as_str()) {
                            continue;
                        }
                    }
                    new_arg_suggestions.push(sugg);
                }

                // Short args
                let short_args = find_argument_suggestions(base_cmd_for_suggestions, "-", span_start, pos)
                    .into_iter()
                    .filter(|s| s.value.starts_with('-') && !s.value.starts_with("--"));

                for sugg in short_args {
                    if let Some(arg_def) = base_cmd_for_suggestions.get_arguments().find(|a| {
                        a.get_short().map_or(false, |s_char| format!("-{}", s_char) == sugg.value)
                    }) {
                        if matches!(*arg_def.get_action(), clap::ArgAction::SetTrue) && existing_flags.contains(sugg.value.as_str()) {
                            continue;
                        }
                    }
                    new_arg_suggestions.push(sugg);
                }
                suggestions.extend(new_arg_suggestions);
            }
        }

        // Deduplicate suggestions
        let mut final_suggestions = Vec::new();
        let mut seen_values = std::collections::HashSet::new();
        for s in suggestions {
            if seen_values.insert(s.value.clone()) {
                final_suggestions.push(s);
            }
        }
        final_suggestions
    }
}
