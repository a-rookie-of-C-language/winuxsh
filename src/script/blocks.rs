use crate::error::{Result, ShellError};
use crate::script::ScriptState;
use crate::script_utils;
use crate::shell::Shell;

impl Shell {
    pub(crate) fn parse_while_block(
        &self,
        lines: &[String],
        start: usize,
        end: usize,
    ) -> Result<(String, usize, usize, usize)> {
        let header = script_utils::normalize_script_line(&lines[start]);
        let mut condition = header.trim_start_matches("while").trim().to_string();
        let mut body_start = start + 1;

        if condition.ends_with("; do") {
            condition = condition.trim_end_matches("; do").trim().to_string();
        } else if condition.ends_with(" do") {
            condition = condition.trim_end_matches(" do").trim().to_string();
        } else {
            let mut cursor = start + 1;
            while cursor < end {
                let candidate = script_utils::normalize_script_line(&lines[cursor]);
                if candidate.is_empty() || candidate.starts_with('#') {
                    cursor += 1;
                    continue;
                }
                if candidate == "do" {
                    body_start = cursor + 1;
                    break;
                }
                return Err(ShellError::InvalidCommand(
                    "while syntax expects 'do'".to_string(),
                ));
            }
        }

        if condition.ends_with(';') {
            condition.pop();
            condition = condition.trim().to_string();
        }
        if condition.is_empty() {
            return Err(ShellError::InvalidCommand(
                "while condition cannot be empty".to_string(),
            ));
        }

        let mut depth = 1usize;
        let mut cursor = body_start;
        while cursor < end {
            let candidate = script_utils::normalize_script_line(&lines[cursor]);
            if candidate.starts_with("while ") {
                depth += 1;
            } else if candidate == "done" {
                depth -= 1;
                if depth == 0 {
                    let body_end = cursor;
                    let next_index = cursor + 1;
                    return Ok((condition, body_start, body_end, next_index));
                }
            }
            cursor += 1;
        }

        Err(ShellError::InvalidCommand(
            "while block missing 'done'".to_string(),
        ))
    }

    pub(crate) fn execute_case_block(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        state: &mut ScriptState,
    ) -> Result<usize> {
        let header = script_utils::normalize_script_line(&lines[start]);
        if !header.ends_with(" in") {
            return Err(ShellError::InvalidCommand(
                "case syntax expects 'case <word> in'".to_string(),
            ));
        }

        let word_expr = header
            .trim_start_matches("case")
            .trim_end_matches(" in")
            .trim();
        let word_expanded = self.expand_script_vars(word_expr, state);
        let case_word = script_utils::strip_quotes(word_expanded.trim());

        let mut depth = 1usize;
        let mut esac_index = None;
        let mut i = start + 1;
        while i < end {
            let line = script_utils::normalize_script_line(&lines[i]);
            if line.starts_with("case ") {
                depth += 1;
            } else if line == "esac" {
                depth -= 1;
                if depth == 0 {
                    esac_index = Some(i);
                    break;
                }
            }
            i += 1;
        }

        let esac = esac_index
            .ok_or_else(|| ShellError::InvalidCommand("case block missing 'esac'".to_string()))?;

        let mut cursor = start + 1;
        let mut matched = false;
        while cursor < esac {
            let line = script_utils::normalize_script_line(&lines[cursor]);
            if line.is_empty() || line.starts_with('#') {
                cursor += 1;
                continue;
            }

            if let Some(close_paren) = line.find(')') {
                let patterns = line[..close_paren].trim();
                let remainder = line[close_paren + 1..].trim();
                let is_match = !matched && script_utils::case_pattern_matches(patterns, &case_word);

                if remainder.ends_with(";;") {
                    if is_match {
                        let cmd = remainder.trim_end_matches(";;").trim();
                        if !cmd.is_empty() {
                            self.execute_script_simple_line(cmd, state)?;
                        }
                        matched = true;
                    }
                    cursor += 1;
                    continue;
                }

                cursor += 1;
                while cursor < esac {
                    let branch_line = script_utils::normalize_script_line(&lines[cursor]);
                    if branch_line.ends_with(";;") {
                        if is_match {
                            let cmd = branch_line.trim_end_matches(";;").trim();
                            if !cmd.is_empty() {
                                self.execute_script_simple_line(cmd, state)?;
                            }
                            matched = true;
                        }
                        break;
                    }

                    if is_match {
                        self.execute_script_simple_line(&branch_line, state)?;
                    }
                    cursor += 1;
                }
            }
            cursor += 1;
        }

        Ok(esac + 1)
    }

    pub(crate) fn execute_if_block(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        state: &mut ScriptState,
    ) -> Result<usize> {
        let mut depth = 0usize;
        let mut fi_index = None;
        let mut i = start;
        while i < end {
            let line = script_utils::normalize_script_line(&lines[i]);
            if line.starts_with("if ") {
                depth += 1;
            } else if line == "fi" {
                depth -= 1;
                if depth == 0 {
                    fi_index = Some(i);
                    break;
                }
            }
            i += 1;
        }

        let fi = fi_index
            .ok_or_else(|| ShellError::InvalidCommand("if block missing 'fi'".to_string()))?;

        let mut branches: Vec<(String, usize, usize)> = Vec::new();
        let mut else_block: Option<(usize, usize)> = None;
        let mut cursor = start;
        let mut nesting = 0usize;
        while cursor <= fi {
            let line = script_utils::normalize_script_line(&lines[cursor]);
            if line.starts_with("if ") {
                nesting += 1;
                if nesting == 1 {
                    let cond = extract_if_condition(&line, "if")?;
                    let (body_start, next) = self.find_then_body_start(lines, cursor, fi)?;
                    let body_end =
                        self.find_if_branch_end(lines, body_start, fi, &["elif", "else", "fi"])?;
                    branches.push((cond, body_start, body_end));
                    cursor = next.max(body_end);
                    continue;
                }
            } else if line == "fi" {
                nesting = nesting.saturating_sub(1);
            } else if nesting == 1 && line.starts_with("elif ") {
                let cond = extract_if_condition(&line, "elif")?;
                let (body_start, next) = self.find_then_body_start(lines, cursor, fi)?;
                let body_end =
                    self.find_if_branch_end(lines, body_start, fi, &["elif", "else", "fi"])?;
                branches.push((cond, body_start, body_end));
                cursor = next.max(body_end);
                continue;
            } else if nesting == 1 && line == "else" {
                else_block = Some((cursor + 1, fi));
                break;
            }
            cursor += 1;
        }

        let mut executed = false;
        for (condition, body_start, body_end) in branches {
            let expanded_condition = self.expand_script_vars(&condition, state);
            self.execute_command(&expanded_condition)?;
            if self.last_exit_code == 0 {
                let _ = self.execute_script_lines(lines, body_start, body_end, state)?;
                executed = true;
                break;
            }
        }

        if !executed {
            if let Some((body_start, body_end)) = else_block {
                let _ = self.execute_script_lines(lines, body_start, body_end, state)?;
            }
        }

        Ok(fi + 1)
    }

    fn find_then_body_start(
        &self,
        lines: &[String],
        start: usize,
        fi: usize,
    ) -> Result<(usize, usize)> {
        let header = script_utils::normalize_script_line(&lines[start]);
        if header.ends_with("; then") || header.ends_with(" then") {
            return Ok((start + 1, start + 1));
        }
        let mut i = start + 1;
        while i <= fi {
            let line = script_utils::normalize_script_line(&lines[i]);
            if line == "then" {
                return Ok((i + 1, i + 1));
            }
            i += 1;
        }
        Err(ShellError::InvalidCommand(
            "if syntax expects 'then'".to_string(),
        ))
    }

    fn find_if_branch_end(
        &self,
        lines: &[String],
        start: usize,
        fi: usize,
        branch_markers: &[&str],
    ) -> Result<usize> {
        let mut depth = 1usize;
        let mut i = start;
        while i <= fi {
            let line = script_utils::normalize_script_line(&lines[i]);
            if line.starts_with("if ") {
                depth += 1;
            } else if line == "fi" {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            } else if depth == 1
                && branch_markers
                    .iter()
                    .any(|m| line == *m || line.starts_with(&format!("{m} ")))
            {
                return Ok(i);
            }
            i += 1;
        }
        Err(ShellError::InvalidCommand(
            "if branch termination not found".to_string(),
        ))
    }
}

fn extract_if_condition(line: &str, keyword: &str) -> Result<String> {
    let raw = line.trim_start_matches(keyword).trim();
    let cond = raw
        .trim_end_matches("; then")
        .trim_end_matches(" then")
        .trim_end_matches(';')
        .trim();
    if cond.is_empty() {
        return Err(ShellError::InvalidCommand(format!(
            "{} condition cannot be empty",
            keyword
        )));
    }
    Ok(cond.to_string())
}
