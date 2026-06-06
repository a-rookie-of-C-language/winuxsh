use std::path::Path;

pub struct PromptContext<'a> {
    pub user: &'a str,
    pub host: &'a str,
    pub dir: &'a str,
    pub last_exit_code: i32,
}

pub fn display_dir(current_dir: &Path, home_dir: Option<&Path>) -> String {
    let dir = current_dir.display().to_string();
    let Some(home_dir) = home_dir else {
        return dir;
    };

    let home = home_dir.display().to_string();
    if dir == home {
        "~".to_string()
    } else if dir.starts_with(&home) {
        format!("~{}", &dir[home.len()..])
    } else {
        dir
    }
}

/// Render a zsh-style prompt template: %F{color}, %f, %n, %m, %~, %#, %T, %?.
pub fn render_template(template: &str, context: &PromptContext<'_>) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            result.push(c);
            continue;
        }

        match chars.peek().copied() {
            Some('F') => {
                chars.next();
                if chars.peek() == Some(&'{') {
                    chars.next();
                    let mut color = String::new();
                    while let Some(&cc) = chars.peek() {
                        if cc == '}' {
                            chars.next();
                            break;
                        }
                        if let Some(next) = chars.next() {
                            color.push(next);
                        }
                    }
                    result.push_str(&ansi_fg(&color));
                }
            }
            Some('f') => {
                chars.next();
                result.push_str("\x1b[39m");
            }
            Some('n') => {
                chars.next();
                result.push_str(context.user);
            }
            Some('m') => {
                chars.next();
                result.push_str(context.host);
            }
            Some('~') => {
                chars.next();
                result.push_str(context.dir);
            }
            Some('#') => {
                chars.next();
                result.push('%');
            }
            Some('T') => {
                chars.next();
                result.push_str(&current_hhmm());
            }
            Some('?') => {
                chars.next();
                result.push_str(&context.last_exit_code.to_string());
            }
            Some(c2) => {
                chars.next();
                result.push('%');
                result.push(c2);
            }
            None => result.push('%'),
        }
    }

    result
}

fn ansi_fg(color: &str) -> String {
    match color.to_lowercase().as_str() {
        "black" => "\x1b[30m".to_string(),
        "red" => "\x1b[31m".to_string(),
        "green" => "\x1b[32m".to_string(),
        "yellow" => "\x1b[33m".to_string(),
        "blue" => "\x1b[34m".to_string(),
        "magenta" => "\x1b[35m".to_string(),
        "cyan" => "\x1b[36m".to_string(),
        "white" => "\x1b[37m".to_string(),
        _ if color.parse::<u8>().is_ok() => format!("\x1b[38;5;{}m", color),
        _ => String::new(),
    }
}

fn current_hhmm() -> String {
    use std::time::SystemTime;

    let Ok(dur) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return String::new();
    };

    let secs = dur.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    format!("{:02}:{:02}", h, m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn render_template_expands_common_prompt_sequences() {
        let context = PromptContext {
            user: "alice",
            host: "host",
            dir: "~/repo",
            last_exit_code: 7,
        };

        let rendered = render_template("%F{red}%n@%m %~ %? %#%f", &context);

        assert_eq!(rendered, "\x1b[31malice@host ~/repo 7 %\x1b[39m");
    }

    #[test]
    fn display_dir_uses_tilde_for_home() {
        let home = PathBuf::from("C:\\Users\\alice");
        let current = home.join("repo");

        assert_eq!(display_dir(&current, Some(&home)), "~\\repo");
        assert_eq!(display_dir(&home, Some(&home)), "~");
    }
}
