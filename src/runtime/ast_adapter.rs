use crate::tokenizer::{CommandInfo, ParsedCommand};
use winsh_ast::redir::{RedirOp, RedirTarget};
use winsh_ast::word::WordPart;
use winsh_ast::{Stmt, Word};

/// Convert parser AST statements to the legacy command model used by the executor.
///
/// This is a compatibility boundary while execution is still based on
/// `ParsedCommand`. Keeping it isolated makes the eventual AST-native executor
/// migration easier to reason about.
pub fn convert_to_parsed_command(
    stmts: &[Stmt],
    resolve_var: &dyn Fn(&str) -> Option<String>,
) -> ParsedCommand {
    if stmts.is_empty() {
        return ParsedCommand::Single(CommandInfo::default());
    }
    if stmts.len() == 1 {
        convert_stmt(&stmts[0], resolve_var)
    } else {
        let cmds: Vec<ParsedCommand> = stmts
            .iter()
            .map(|stmt| convert_stmt(stmt, resolve_var))
            .collect();
        ParsedCommand::Sequence(cmds)
    }
}

fn convert_stmt(stmt: &Stmt, resolve_var: &dyn Fn(&str) -> Option<String>) -> ParsedCommand {
    match stmt {
        Stmt::Command {
            words,
            redirections,
            background,
        } => {
            let args: Vec<String> = words
                .iter()
                .map(|word| expand_word(word, resolve_var))
                .collect();
            let mut cmd = CommandInfo {
                args,
                background: *background,
                ..Default::default()
            };

            for redir in redirections {
                apply_redirection(&mut cmd, redir.op, &redir.target, resolve_var);
            }

            ParsedCommand::Single(cmd)
        }
        Stmt::Pipeline {
            commands,
            negated: _,
        } => {
            let cmds: Vec<CommandInfo> = commands
                .iter()
                .filter_map(|stmt| {
                    if let Stmt::Command { words, .. } = stmt {
                        Some(CommandInfo {
                            args: words
                                .iter()
                                .map(|word| expand_word(word, resolve_var))
                                .collect(),
                            ..Default::default()
                        })
                    } else {
                        None
                    }
                })
                .collect();
            ParsedCommand::Pipeline(cmds)
        }
        Stmt::And { left, right } => ParsedCommand::And(
            Box::new(convert_stmt(left, resolve_var)),
            Box::new(convert_stmt(right, resolve_var)),
        ),
        Stmt::Or { left, right } => ParsedCommand::Or(
            Box::new(convert_stmt(left, resolve_var)),
            Box::new(convert_stmt(right, resolve_var)),
        ),
        Stmt::Sequence(stmts) => {
            let cmds: Vec<ParsedCommand> = stmts
                .iter()
                .map(|stmt| convert_stmt(stmt, resolve_var))
                .collect();
            ParsedCommand::Sequence(cmds)
        }
        _ => ParsedCommand::Single(CommandInfo::default()),
    }
}

fn apply_redirection(
    cmd: &mut CommandInfo,
    op: RedirOp,
    target: &RedirTarget,
    resolve_var: &dyn Fn(&str) -> Option<String>,
) {
    match op {
        RedirOp::In => {
            if let RedirTarget::File(word) = target {
                cmd.stdin_redir = Some(expand_word(word, resolve_var));
            }
        }
        RedirOp::Out => {
            if let RedirTarget::File(word) = target {
                cmd.stdout_redir = Some(expand_word(word, resolve_var));
            }
        }
        RedirOp::Append => {
            if let RedirTarget::File(word) = target {
                cmd.stdout_redir = Some(expand_word(word, resolve_var));
                cmd.stdout_append = true;
            }
        }
        RedirOp::Err => {
            if let RedirTarget::File(word) = target {
                cmd.stderr_redir = Some(expand_word(word, resolve_var));
            }
        }
        RedirOp::ErrAppend => {
            if let RedirTarget::File(word) = target {
                cmd.stderr_redir = Some(expand_word(word, resolve_var));
                cmd.stderr_append = true;
            }
        }
        RedirOp::ErrToOut => {
            cmd.stderr_to_stdout = true;
        }
        RedirOp::OutToErr => {
            cmd.stdout_to_stderr = true;
        }
        _ => {}
    }
}

fn expand_word(word: &Word, resolve_var: &dyn Fn(&str) -> Option<String>) -> String {
    let mut result = String::new();
    for part in &word.parts {
        match part {
            WordPart::Literal(s) => result.push_str(s),
            WordPart::Variable(name) => {
                result.push_str(&resolve_var(name).unwrap_or_default());
            }
            WordPart::BracedVariable(spec) => {
                let var_name = spec.trim_matches(|c: char| c == '{' || c == '}');
                result.push_str(&resolve_var(var_name).unwrap_or_default());
            }
            WordPart::SingleQuoted(s) => result.push_str(s),
            WordPart::DollarQuoted(s) => result.push_str(s),
            WordPart::DoubleQuoted(inner) => {
                for part in inner {
                    match part {
                        WordPart::Literal(s) => result.push_str(s),
                        WordPart::Variable(name) => {
                            result.push_str(&resolve_var(name).unwrap_or_default());
                        }
                        WordPart::BracedVariable(spec) => {
                            let var_name = spec.trim_matches(|c: char| c == '{' || c == '}');
                            result.push_str(&resolve_var(var_name).unwrap_or_default());
                        }
                        _ => result.push_str(&part.to_string()),
                    }
                }
            }
            _ => result.push_str(&part.to_string()),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsh_lexer::Lexer;
    use winsh_parser::Parser;

    fn convert(input: &str) -> ParsedCommand {
        let tokens = Lexer::tokenize(input).unwrap();
        let stmts = Parser::parse(tokens).unwrap();
        convert_to_parsed_command(&stmts, &|_| None)
    }

    #[test]
    fn converts_fd_redirections() {
        let parsed = convert("echo hello 2> err.txt 1>&2");
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert_eq!(cmd.args, vec!["echo", "hello"]);
                assert_eq!(cmd.stderr_redir.as_deref(), Some("err.txt"));
                assert!(cmd.stdout_to_stderr);
            }
            _ => panic!("expected single command"),
        }
    }

    #[test]
    fn resolves_variables_through_injected_resolver() {
        let tokens = Lexer::tokenize("echo $LOCAL").unwrap();
        let stmts = Parser::parse(tokens).unwrap();
        let parsed = convert_to_parsed_command(&stmts, &|name| {
            (name == "LOCAL").then(|| "from-shell".to_string())
        });

        match parsed {
            ParsedCommand::Single(cmd) => {
                assert_eq!(cmd.args, vec!["echo", "from-shell"]);
            }
            _ => panic!("expected single command"),
        }
    }
}
