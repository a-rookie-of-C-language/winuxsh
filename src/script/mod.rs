use std::collections::HashMap;
use std::path::Path;

use crate::array::ArrayValue;
use crate::error::{Result, ShellError};
use crate::script_utils;
use crate::shell::Shell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptFlow {
    None,
    Break,
    Continue,
}

#[derive(Debug, Default)]
pub(crate) struct ScriptState {
    pub(crate) positional: Vec<String>,
    pub(crate) locals: HashMap<String, String>,
    functions: HashMap<String, Vec<String>>,
}

impl ScriptState {
    fn new(args: &[String]) -> Self {
        Self {
            positional: args.to_vec(),
            locals: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn shift(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if n >= self.positional.len() {
            self.positional.clear();
            return;
        }
        self.positional.drain(0..n);
    }

    pub(crate) fn positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|s| s.as_str())
    }
}

impl Shell {
    /// Execute a script file with basic script semantics and positional args.
    pub fn run_script_file(&mut self, script_path: &Path, script_args: &[String]) -> Result<()> {
        let script_content = std::fs::read_to_string(script_path)?;
        let lines: Vec<String> = script_content.lines().map(|s| s.to_string()).collect();
        let mut state = ScriptState::new(script_args);
        let _ = self.execute_script_lines(&lines, 0, lines.len(), &mut state)?;
        Ok(())
    }

    pub(crate) fn execute_script_lines(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        state: &mut ScriptState,
    ) -> Result<ScriptFlow> {
        let mut i = start;
        while i < end {
            let line = script_utils::normalize_script_line(&lines[i]);
            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }

            if line == "break" {
                return Ok(ScriptFlow::Break);
            }
            if line == "continue" {
                return Ok(ScriptFlow::Continue);
            }

            if line.starts_with("while ") {
                let (condition, body_start, body_end, next_index) =
                    self.parse_while_block(lines, i, end)?;
                loop {
                    let expanded_condition = self.expand_script_vars(&condition, state);
                    self.execute_command(&expanded_condition)?;
                    if self.last_exit_code != 0 {
                        break;
                    }

                    match self.execute_script_lines(lines, body_start, body_end, state)? {
                        ScriptFlow::None => {}
                        ScriptFlow::Break => break,
                        ScriptFlow::Continue => continue,
                    }
                }
                i = next_index;
                continue;
            }

            if line.starts_with("if ") {
                i = self.execute_if_block(lines, i, end, state)?;
                continue;
            }

            if line.starts_with("case ") {
                i = self.execute_case_block(lines, i, end, state)?;
                continue;
            }

            if let Some(func_name) = script_utils::parse_function_header(&line) {
                i = self.register_script_function(lines, i, end, func_name, state)?;
                continue;
            }

            self.execute_script_simple_line(&line, state)?;
            i += 1;
        }

        Ok(ScriptFlow::None)
    }

    fn register_script_function(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        func_name: String,
        state: &mut ScriptState,
    ) -> Result<usize> {
        let mut depth = 1usize;
        let mut cursor = start + 1;
        let mut body: Vec<String> = Vec::new();
        while cursor < end {
            let line = script_utils::normalize_script_line(&lines[cursor]);
            if line.ends_with('{') {
                depth += 1;
            } else if line == "}" {
                depth -= 1;
                if depth == 0 {
                    state.functions.insert(func_name, body);
                    return Ok(cursor + 1);
                }
            }
            if depth > 0 {
                body.push(line);
            }
            cursor += 1;
        }

        Err(ShellError::InvalidCommand(
            "function block missing '}'".to_string(),
        ))
    }

    pub(crate) fn execute_script_simple_line(
        &mut self,
        line: &str,
        state: &mut ScriptState,
    ) -> Result<()> {
        let trimmed = line.trim();
        if trimmed.starts_with("unset -f ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() > 2 {
                state.functions.remove(parts[2]);
            }
            return Ok(());
        }

        if trimmed.starts_with("shift") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let shift_n = if parts.len() > 1 {
                parts[1].parse::<usize>().unwrap_or(1)
            } else {
                1
            };
            state.shift(shift_n);
            return Ok(());
        }

        let expanded = self.expand_script_vars(trimmed, state);
        if expanded.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = expanded.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mut idx = 0usize;
        while idx < parts.len() && script_utils::is_assignment_token(parts[idx]) {
            if let Some((key, value)) = parts[idx].split_once('=') {
                let mut normalized_value = script_utils::normalize_assignment_value(value);
                if key.eq_ignore_ascii_case("PATH") {
                    normalized_value =
                        script_utils::normalize_path_list_for_windows(&normalized_value);
                }

                state
                    .locals
                    .insert(key.to_string(), normalized_value.to_string());
                self.env_vars.insert(
                    key.to_string(),
                    ArrayValue::String(normalized_value.clone()),
                );
            }
            idx += 1;
        }

        if idx >= parts.len() {
            return Ok(());
        }

        if let Some(func_body) = state.functions.get(parts[idx]).cloned() {
            let _ = self.execute_script_lines(&func_body, 0, func_body.len(), state)?;
            return Ok(());
        }

        let command = parts[idx..].join(" ");
        self.execute_command(&command)
    }
}
