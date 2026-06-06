use std::path::Path;

use crate::capture::TempCapture;
use crate::shell::Shell;
use crate::tokenizer::{CommandInfo, ParsedCommand};
use winsh_lexer::Lexer;
use winsh_parser::Parser;

impl Shell {
    /// Expand wildcards in arguments.
    pub fn expand_wildcards(&self, args: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();

        for arg in args {
            if arg.contains('*') || arg.contains('?') || arg.contains('[') {
                let arg_path = Path::new(arg);
                let pattern = if arg_path.is_absolute() {
                    arg.clone()
                } else {
                    self.current_dir
                        .join(arg_path)
                        .to_string_lossy()
                        .to_string()
                };

                if let Ok(matches) = glob::glob(&pattern) {
                    let mut matched_paths = Vec::new();
                    for entry in matches.flatten() {
                        if arg_path.is_absolute() {
                            matched_paths.push(entry.to_string_lossy().to_string());
                        } else if let Ok(relative) = entry.strip_prefix(&self.current_dir) {
                            matched_paths.push(relative.to_string_lossy().to_string());
                        } else {
                            matched_paths.push(entry.to_string_lossy().to_string());
                        }
                    }

                    if matched_paths.is_empty() {
                        expanded.push(arg.clone());
                    } else {
                        expanded.extend(matched_paths);
                    }
                } else {
                    expanded.push(arg.clone());
                }
            } else {
                expanded.push(arg.clone());
            }
        }

        expanded
    }

    /// Expand command substitution $(...).
    pub fn expand_command_substitution(&mut self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&'(') = chars.peek() {
                    chars.next();
                    let mut command = String::new();
                    let mut depth = 1;

                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '(' {
                            depth += 1;
                            command.push(c);
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            } else {
                                command.push(c);
                            }
                        } else {
                            command.push(c);
                        }
                    }

                    let output = self.execute_substitution_command(&command);
                    result.push_str(output.trim());
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    pub(crate) fn execute_substitution_command(&mut self, command: &str) -> String {
        let tokens = match Lexer::tokenize(command) {
            Ok(tokens) => tokens,
            Err(_) => return String::new(),
        };

        let stmts = match Parser::parse(tokens) {
            Ok(stmts) => stmts,
            Err(_) => return String::new(),
        };

        let parsed = crate::ast_adapter::convert_to_parsed_command(&stmts, &|name| {
            self.resolve_env_var(name)
        });
        let capture = TempCapture::new("winuxsh_subst");
        let redirected =
            self.redirect_parsed_for_capture(&parsed, capture.stdout_path(), capture.stderr_path());
        let _ = self.execute_parsed(&redirected);

        std::fs::read_to_string(capture.stdout_path()).unwrap_or_default()
    }

    fn redirect_parsed_for_capture(
        &self,
        parsed: &ParsedCommand,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> ParsedCommand {
        match parsed {
            ParsedCommand::Single(cmd) => ParsedCommand::Single(
                self.redirect_command_info_for_capture(cmd, stdout_path, stderr_path, true),
            ),
            ParsedCommand::Pipeline(cmds) => {
                let mut redirected = cmds.clone();
                if let Some((last, prefix)) = redirected.split_last_mut() {
                    for cmd in prefix.iter_mut() {
                        if cmd.stderr_redir.is_none() {
                            cmd.stderr_redir = Some(stderr_path.to_string_lossy().to_string());
                            cmd.stderr_append = true;
                        }
                    }
                    *last = self.redirect_command_info_for_capture(
                        last,
                        stdout_path,
                        stderr_path,
                        true,
                    );
                }
                ParsedCommand::Pipeline(redirected)
            }
            ParsedCommand::And(left, right) => ParsedCommand::And(
                Box::new(self.redirect_parsed_for_capture(left, stdout_path, stderr_path)),
                Box::new(self.redirect_parsed_for_capture(right, stdout_path, stderr_path)),
            ),
            ParsedCommand::Or(left, right) => ParsedCommand::Or(
                Box::new(self.redirect_parsed_for_capture(left, stdout_path, stderr_path)),
                Box::new(self.redirect_parsed_for_capture(right, stdout_path, stderr_path)),
            ),
            ParsedCommand::Sequence(commands) => ParsedCommand::Sequence(
                commands
                    .iter()
                    .map(|command| {
                        self.redirect_parsed_for_capture(command, stdout_path, stderr_path)
                    })
                    .collect(),
            ),
        }
    }

    fn redirect_command_info_for_capture(
        &self,
        cmd: &CommandInfo,
        stdout_path: &Path,
        stderr_path: &Path,
        capture_stdout: bool,
    ) -> CommandInfo {
        let mut redirected = cmd.clone();

        if capture_stdout && redirected.stdout_redir.is_none() {
            redirected.stdout_redir = Some(stdout_path.to_string_lossy().to_string());
            redirected.stdout_append = true;
        }

        if redirected.stderr_redir.is_none() {
            redirected.stderr_redir = Some(stderr_path.to_string_lossy().to_string());
            redirected.stderr_append = true;
        }

        redirected
    }
}
