use crate::script::ScriptState;
use crate::shell::Shell;

impl Shell {
    pub(crate) fn expand_script_vars(&self, input: &str, state: &ScriptState) -> String {
        let mut out = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch != '$' {
                out.push(ch);
                continue;
            }

            let Some(next) = chars.peek().copied() else {
                out.push('$');
                break;
            };

            match next {
                '{' => {
                    chars.next();
                    let mut name = String::new();
                    while let Some(c) = chars.peek().copied() {
                        chars.next();
                        if c == '}' {
                            break;
                        }
                        name.push(c);
                    }
                    if let Some((var_name, default_value)) = name.split_once(":-") {
                        let value = self.resolve_script_var(var_name.trim(), state);
                        if value.is_empty() {
                            out.push_str(default_value);
                        } else {
                            out.push_str(&value);
                        }
                    } else {
                        out.push_str(&self.resolve_script_var(&name, state));
                    }
                }
                '#' => {
                    chars.next();
                    out.push_str(&state.positional.len().to_string());
                }
                '@' | '*' => {
                    chars.next();
                    out.push_str(&state.positional.join(" "));
                }
                c if c.is_ascii_digit() => {
                    let mut index = String::new();
                    while let Some(d) = chars.peek().copied() {
                        if d.is_ascii_digit() {
                            chars.next();
                            index.push(d);
                        } else {
                            break;
                        }
                    }
                    let idx = index.parse::<usize>().unwrap_or(0);
                    if idx > 0 {
                        if let Some(value) = state.positional(idx - 1) {
                            out.push_str(value);
                        }
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut name = String::new();
                    while let Some(c2) = chars.peek().copied() {
                        if c2.is_ascii_alphanumeric() || c2 == '_' {
                            chars.next();
                            name.push(c2);
                        } else {
                            break;
                        }
                    }
                    out.push_str(&self.resolve_script_var(&name, state));
                }
                _ => out.push('$'),
            }
        }

        out
    }

    fn resolve_script_var(&self, name: &str, state: &ScriptState) -> String {
        if let Some(value) = state.locals.get(name) {
            return value.clone();
        }

        if let Some(value) = self.env_vars.get(name) {
            if let Some(s) = value.as_string() {
                return s.to_string();
            }
        }

        if let Some((_, value)) = self
            .env_vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
        {
            if let Some(s) = value.as_string() {
                return s.to_string();
            }
        }

        String::new()
    }
}
