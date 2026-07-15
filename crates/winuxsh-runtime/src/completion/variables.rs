// Variable completion for WinSH
// Provides Tab completion for environment variables

use std::collections::HashMap;

use anyhow::Result;

use crate::completion::{CompletionContext, CompletionResult};

/// Variable completer
pub struct VariableCompleter;

impl VariableCompleter {
    /// Complete a variable name
    pub fn complete(
        context: &CompletionContext,
        env_vars: &HashMap<String, String>,
    ) -> Result<Option<CompletionResult>> {
        let word = match context.get_current_word() {
            Some(w) => w,
            None => return Ok(None),
        };

        if !word.starts_with('$') {
            return Ok(None);
        }

        let var_name = if let Some(rest) = word.strip_prefix("${") {
            rest.strip_suffix('}').unwrap_or(rest)
        } else if let Some(rest) = word.strip_prefix('$') {
            rest
        } else {
            return Ok(None);
        };

        let mut all_vars = Self::get_environment_variables();

        for key in env_vars.keys() {
            all_vars.push(key.clone());
        }

        let matches: Vec<String> = all_vars
            .into_iter()
            .filter(|var| var.to_lowercase().starts_with(&var_name.to_lowercase()))
            .map(|var| {
                if word.starts_with("${") {
                    format!("${{{}}}", var)
                } else {
                    format!("${}", var)
                }
            })
            .collect();

        if matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResult::new(matches)))
        }
    }

    /// Get system environment variables
    pub fn get_environment_variables() -> Vec<String> {
        std::env::vars().map(|(key, _)| key).collect()
    }

    /// Get common environment variables for quick completion
    pub fn get_common_variables() -> Vec<String> {
        vec![
            "HOME".to_string(),
            "USER".to_string(),
            "PATH".to_string(),
            "PWD".to_string(),
            "SHELL".to_string(),
            "TERM".to_string(),
            "LANG".to_string(),
            "LC_ALL".to_string(),
            "EDITOR".to_string(),
            "VISUAL".to_string(),
            "PAGER".to_string(),
            "PS1".to_string(),
            "PS2".to_string(),
            "HOSTNAME".to_string(),
            "HOSTTYPE".to_string(),
            "OSTYPE".to_string(),
            "MACHTYPE".to_string(),
            "SHLVL".to_string(),
            "LOGNAME".to_string(),
        ]
    }

    /// Expand environment variables in a string
    pub fn expand_variables(input: &str, env_vars: &HashMap<String, String>) -> String {
        let mut result = input.to_string();

        while let Some(start) = result.find('$') {
            let rest = &result[start + 1..];

            if rest.starts_with('{') {
                if let Some(end) = rest.find('}') {
                    let var_name = &rest[1..end];
                    let replacement = Self::get_variable_value(var_name, env_vars);
                    result = format!("{}{}{}", &result[..start], replacement, &rest[end + 1..]);
                    continue;
                }
            }

            let end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());

            let var_name = &rest[..end];
            let replacement = Self::get_variable_value(var_name, env_vars);
            result = format!("{}{}{}", &result[..start], replacement, &rest[end..]);
        }

        result
    }

    /// Get the value of a variable
    fn get_variable_value(var_name: &str, env_vars: &HashMap<String, String>) -> String {
        if let Some(value) = env_vars.get(var_name) {
            return value.clone();
        }

        if let Ok(value) = std::env::var(var_name) {
            return value;
        }

        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_environment_variables() {
        std::env::set_var("WINUXSH_TEST_ENV", "1");
        let vars = VariableCompleter::get_environment_variables();
        assert!(vars.iter().any(|k| k == "WINUXSH_TEST_ENV"));
    }

    #[test]
    fn test_get_common_variables() {
        let vars = VariableCompleter::get_common_variables();
        assert!(vars.contains(&"PATH".to_string()));
        assert!(vars.contains(&"HOME".to_string()));
    }

    #[test]
    fn test_expand_variables() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST".to_string(), "value".to_string());

        let result = VariableCompleter::expand_variables("echo $TEST", &env_vars);
        assert!(result.contains("echo"));
    }
}
