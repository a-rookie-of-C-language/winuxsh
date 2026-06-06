pub fn normalize_script_line(line: &str) -> String {
    let trimmed =
        line.trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace());
    strip_inline_comment(trimmed).trim().to_string()
}

pub fn strip_inline_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single {
            out.push(ch);
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            out.push(ch);
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            out.push(ch);
            continue;
        }

        if ch == '#' && !in_single && !in_double {
            break;
        }

        out.push(ch);
    }

    out
}

pub fn strip_quotes(s: &str) -> String {
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\'')
        {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

pub fn normalize_assignment_value(raw: &str) -> String {
    let mut out = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for ch in raw.chars() {
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }

        out.push(ch);
    }

    out
}

pub fn normalize_path_list_for_windows(path_value: &str) -> String {
    if cfg!(not(windows)) || path_value.is_empty() {
        return path_value.to_string();
    }

    if path_value.contains(';') {
        return path_value.to_string();
    }

    let chars: Vec<char> = path_value.chars().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();

    for i in 0..chars.len() {
        let ch = chars[i];
        if ch == ':' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                None
            };
            let is_drive_colon = prev.map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
                && next.map(|c| c == '\\' || c == '/').unwrap_or(false);
            if !is_drive_colon {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                } else {
                    parts.push(String::new());
                }
                continue;
            }
        }
        current.push(ch);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts.join(";")
}

pub fn parse_function_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.ends_with('{') {
        return None;
    }
    let header = trimmed.trim_end_matches('{').trim();
    if let Some(name) = header.strip_suffix("()") {
        let n = name.trim();
        if !n.is_empty() {
            return Some(n.to_string());
        }
    }
    None
}

pub fn case_pattern_matches(patterns: &str, value: &str) -> bool {
    for pattern in patterns.split('|') {
        let pat = strip_quotes(pattern.trim());
        if pat == "*" {
            return true;
        }
        if pat.contains('*') || pat.contains('?') || pat.contains('[') {
            if let Ok(glob_pattern) = glob::Pattern::new(&pat) {
                if glob_pattern.matches(value) {
                    return true;
                }
            }
        } else if pat == value {
            return true;
        }
    }
    false
}

pub fn is_assignment_token(token: &str) -> bool {
    if let Some((key, _)) = token.split_once('=') {
        if key.is_empty() {
            return false;
        }
        let mut chars = key.chars();
        if let Some(first) = chars.next() {
            if !(first.is_ascii_alphabetic() || first == '_') {
                return false;
            }
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_script_line_keeps_quoted_hashes() {
        assert_eq!(
            normalize_script_line(" echo '#not comment' # comment "),
            "echo '#not comment'"
        );
        assert_eq!(
            normalize_script_line("echo \"#also text\""),
            "echo \"#also text\""
        );
    }

    #[test]
    fn assignment_detection_matches_shell_identifiers() {
        assert!(is_assignment_token("PATH=value"));
        assert!(is_assignment_token("_X=1"));
        assert!(!is_assignment_token("1X=value"));
        assert!(!is_assignment_token("echo"));
    }

    #[test]
    fn normalize_assignment_value_removes_quote_characters() {
        assert_eq!(normalize_assignment_value("\"hello\""), "hello");
        assert_eq!(normalize_assignment_value("'a b'"), "a b");
    }

    #[test]
    fn case_pattern_matching_supports_alternatives_and_globs() {
        assert!(case_pattern_matches("a|b*", "bee"));
        assert!(case_pattern_matches("\"exact\"", "exact"));
        assert!(!case_pattern_matches("a|b", "c"));
    }
}
