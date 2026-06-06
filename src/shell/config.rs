use std::path::{Path, PathBuf};

use crate::array::ArrayValue;
use crate::error::Result;
use crate::shell::Shell;

impl Shell {
    pub fn parse_config_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        self.process_winshrc_content(&content, &home.display().to_string())?;
        Ok(())
    }

    /// Process .winshrc content recursively (handles source, export, alias, setopt, etc.).
    fn process_winshrc_content(&mut self, content: &str, home_str: &str) -> Result<()> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let expanded = self.expand_config_line(trimmed, home_str);

            if let Some(path) = expanded.strip_prefix("source ") {
                let path = path.trim().trim_matches('"').trim_matches('\'');
                let source_path = if path.starts_with("$HOME") || path.starts_with('~') {
                    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                    let home_display = home.display().to_string();
                    let rel =
                        path.replacen("$HOME", &home_display, 1)
                            .replacen('~', &home_display, 1);
                    PathBuf::from(rel.replace('/', "\\"))
                } else {
                    PathBuf::from(path)
                };

                log::debug!("source: {} -> {}", path, source_path.display());
                if source_path.exists() {
                    if let Ok(source_content) = std::fs::read_to_string(&source_path) {
                        log::debug!(
                            "source: loaded {} ({} bytes)",
                            source_path.display(),
                            source_content.len()
                        );
                        self.process_winshrc_content(&source_content, home_str)?;
                    } else {
                        log::warn!("source: failed to read {}", source_path.display());
                    }
                } else {
                    log::warn!("source: file not found: {}", source_path.display());
                }
                continue;
            }

            if let Some(rest) = expanded.strip_prefix("export ") {
                if let Some((name, value)) = rest.split_once('=') {
                    let name = name.trim();
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    self.env_vars.insert(name.to_string(), env_value(value));
                }
                continue;
            }

            if let Some(rest) = expanded.strip_prefix("alias ") {
                if let Some((name, value)) = rest.split_once('=') {
                    let value = value.trim().trim_matches('\'').trim_matches('"');
                    self.aliases
                        .insert(name.trim().to_string(), value.to_string());
                }
                continue;
            }

            if let Some(opt) = expanded.strip_prefix("setopt ") {
                log::debug!("setopt: {}", opt.trim());
                continue;
            }

            if let Some((name, value)) = expanded.split_once('=') {
                let name = name.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    match name {
                        "WINUXSH_THEME" => {
                            log::debug!("config: setting WINUXSH_THEME={}", value);
                            self.env_vars.insert(name.to_string(), env_value(value));
                        }
                        "PROMPT" | "PS1" => {
                            log::debug!("config: setting {}={}", name, value);
                            self.env_vars.insert(name.to_string(), env_value(value));
                        }
                        _ => {
                            self.env_vars.insert(name.to_string(), env_value(value));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Expand variables in a config line ($HOME, ${VAR}, $VAR).
    fn expand_config_line(&self, line: &str, home_str: &str) -> String {
        let mut result = line.to_string();
        result = result.replace("$HOME", home_str);

        let mut vars: Vec<(&String, &str)> = self
            .env_vars
            .iter()
            .filter_map(|(k, v)| v.as_string().map(|s| (k, s)))
            .collect();
        vars.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));

        for (name, value) in &vars {
            result = result.replace(&format!("${{{}}}", name), value);
        }

        for (name, value) in &vars {
            let pattern = format!("${}", name);
            if result.contains(&pattern) {
                log::trace!("expand_config: replacing {} with {}", pattern, value);
            }
            result = result.replace(&pattern, value);
        }

        result
    }

    pub(crate) fn load_config(&mut self) -> Result<()> {
        let winshrc = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".winshrc");
        if winshrc.exists() {
            log::info!("Loading config: {}", winshrc.display());
            self.parse_config_file(&winshrc)?;
            log::info!("Config loaded successfully");
        } else if let Some(config_path) = crate::config::ConfigManager::find_config_file() {
            if config_path
                .extension()
                .map(|e| e == "toml")
                .unwrap_or(false)
            {
                let mut config_manager = crate::config::ConfigManager::new();
                self.config = config_manager.load_config(&config_path)?;
            } else {
                self.parse_config_file(&config_path)?;
            }
        }
        Ok(())
    }
}

fn env_value(v: &str) -> ArrayValue {
    ArrayValue::String(v.to_string())
}
