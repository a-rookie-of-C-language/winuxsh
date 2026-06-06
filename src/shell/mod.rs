use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::array::ArrayValue;
use crate::config::ShellConfig;
use crate::error::Result;
use crate::job::JobManager;
use crate::plugin::PluginManager;
use crate::prompt::{display_dir, render_template, PromptContext};
use crate::theme::ThemePlugin;

/// Main shell structure
pub struct Shell {
    pub current_dir: PathBuf,
    pub aliases: HashMap<String, String>,
    pub env_vars: HashMap<String, ArrayValue>,
    pub line_editor: Reedline,
    pub history_path: PathBuf,
    pub config: ShellConfig,
    pub plugins: PluginManager,
    pub job_manager: JobManager,
    pub theme_plugin: ThemePlugin,
    pub last_exit_code: i32,
    pub command_router: Option<crate::command_router::CommandRouter>,
    // Store completion state reference for updates
    pub(crate) completion_state:
        std::sync::Arc<std::sync::Mutex<crate::completion::CompletionState>>,
}
impl Shell {
    /// Get environment variable
    pub fn get_env_var(&self, key: &str, default: &str) -> String {
        self.resolve_env_var(key)
            .unwrap_or_else(|| default.to_string())
    }

    pub(crate) fn resolve_env_var(&self, key: &str) -> Option<String> {
        if let Some(value) = self.env_vars.get(key) {
            return value.as_string().map(|s| s.to_string());
        }

        self.env_vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .and_then(|(_, v)| v.as_string().map(|s| s.to_string()))
    }

    /// Get the prompt string
    pub fn get_prompt(&self) -> DefaultPrompt {
        let username = self.get_env_var("USERNAME", "user");
        let hostname = self.get_env_var("COMPUTERNAME", "localhost");
        let home_dir = dirs::home_dir();
        let dir_display = display_dir(&self.current_dir, home_dir.as_deref());
        let prompt_context = PromptContext {
            user: &username,
            host: &hostname,
            dir: &dir_display,
            last_exit_code: self.last_exit_code,
        };

        // Check if PROMPT env var is set (oh-my-winuxsh theme)
        let prompt_text = if let Some(ArrayValue::String(ref fmt)) = self.env_vars.get("PROMPT") {
            render_template(fmt, &prompt_context)
        } else if let Some(ArrayValue::String(ref fmt)) = self.env_vars.get("PS1") {
            render_template(fmt, &prompt_context)
        } else {
            let ThemePlugin::Theme(ref theme) = self.theme_plugin;
            theme.generate_prompt(&username, &hostname, &dir_display, "$ ")
        };

        DefaultPrompt::new(
            DefaultPromptSegment::Basic(prompt_text),
            DefaultPromptSegment::Empty,
        )
    }

    /// Save command to history
    pub fn save_history(&mut self, command: &str) -> Result<()> {
        let clean_command =
            command.trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace());

        if clean_command.is_empty() {
            return Ok(());
        }

        let mut history = if self.history_path.exists() {
            std::fs::read_to_string(&self.history_path)?
        } else {
            String::new()
        };

        if !history.is_empty() {
            history.push('\n');
        }
        history.push_str(clean_command);

        std::fs::write(&self.history_path, history)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_shell_creation() {
        let shell = Shell::new(false);
        assert!(shell.is_ok());
        let shell = shell.unwrap();
        assert_eq!(shell.current_dir, std::env::current_dir().unwrap());
    }

    #[test]
    fn test_get_env_var() {
        let shell = Shell::new(false).unwrap();
        let value = shell.get_env_var("USERNAME", "default");
        assert!(value != "default" || std::env::var("USERNAME").is_err());
    }

    #[test]
    fn test_and_short_circuit_on_failure() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_NOT_RUN");
        shell
            .execute_command("notexistcmd && set SHOULD_NOT_RUN=1")
            .unwrap();
        assert!(shell.env_vars.get("SHOULD_NOT_RUN").is_none());
        assert_ne!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_or_runs_on_failure() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_RUN");
        shell
            .execute_command("notexistcmd || set SHOULD_RUN=1")
            .unwrap();
        assert_eq!(
            shell.env_vars.get("SHOULD_RUN"),
            Some(&ArrayValue::String("1".to_string()))
        );
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_or_short_circuit_on_success() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_NOT_RUN_OR");
        shell
            .execute_command("echo hi || set SHOULD_NOT_RUN_OR=1")
            .unwrap();
        assert!(shell.env_vars.get("SHOULD_NOT_RUN_OR").is_none());
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_builtin_echo_redirection() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_redir_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let out_path = test_dir.join("out.txt");
        let cmd = format!("echo hello > {}", out_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let out = fs::read_to_string(out_path).unwrap();
        assert_eq!(out, "hello\n");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_script_control_flow_and_positional_args() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_script_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();

        let script_path = test_dir.join("verify.sh");
        let out_path = test_dir.join("out.txt");
        let script = "\
#!/bin/bash
while true; do
  echo loop
  break
done
RUN_CMD=echo
case x in
  x) echo ok ;;
  *) echo bad ;;
esac
echo first=$1
shift
echo second=$1
echo redirected > __OUT_PATH__
";
        let script = script.replace("__OUT_PATH__", &out_path.to_string_lossy());
        fs::write(&script_path, script).unwrap();

        let args = vec!["A".to_string(), "B".to_string()];
        shell.run_script_file(&script_path, &args).unwrap();

        let out = fs::read_to_string(out_path).unwrap();
        assert_eq!(out, "redirected\n");
        assert_eq!(
            shell.env_vars.get("RUN_CMD"),
            Some(&ArrayValue::String("echo".to_string()))
        );
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_expand_wildcards_keeps_unmatched_pattern_per_argument() {
        let shell = Shell::new(false).unwrap();
        let unmatched = format!("winuxsh_unmatched_{}_*.unlikely", std::process::id());
        let args = vec!["Cargo.*".to_string(), unmatched.clone()];

        let expanded = shell.expand_wildcards(&args);

        assert!(expanded.iter().any(|arg| arg.ends_with("Cargo.toml")));
        assert!(expanded.contains(&unmatched));
    }

    #[test]
    fn test_builtin_stderr_redirection_file_materialized() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_stderr_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let err_path = test_dir.join("err.txt");
        let cmd = format!("echo hello 2> {}", err_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let err = fs::read_to_string(err_path).unwrap();
        assert_eq!(err, "");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_builtin_stdout_append_redirection() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir =
            std::env::temp_dir().join(format!("winuxsh_stdout_append_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let out_path = test_dir.join("out.txt");
        let first = format!("echo one > {}", out_path.to_string_lossy());
        let second = format!("echo two >> {}", out_path.to_string_lossy());

        shell.execute_command(&first).unwrap();
        shell.execute_command(&second).unwrap();

        let out = fs::read_to_string(out_path).unwrap();
        assert_eq!(out, "one\ntwo\n");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_redirection_relative_path_uses_shell_current_dir() {
        let mut shell = Shell::new(false).unwrap();
        let original_dir = std::env::current_dir().unwrap();
        let test_dir =
            std::env::temp_dir().join(format!("winuxsh_redir_cwd_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        shell.current_dir = test_dir.clone();
        let file_name = format!("out_{}.txt", std::process::id());

        shell
            .execute_command(&format!("echo cwd-relative > {}", file_name))
            .unwrap();

        let expected = test_dir.join(&file_name);
        let unexpected = original_dir.join(&file_name);
        assert_eq!(fs::read_to_string(expected).unwrap(), "cwd-relative\n");
        if unexpected.exists() {
            let unexpected_content = fs::read_to_string(&unexpected).unwrap_or_default();
            assert_ne!(unexpected_content, "cwd-relative\n");
        }
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_word_expansion_uses_shell_env_vars() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir =
            std::env::temp_dir().join(format!("winuxsh_shell_env_expand_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let out_path = test_dir.join("out.txt");

        shell.execute_command("set LOCAL_ONLY=from-shell").unwrap();
        shell
            .execute_command(&format!(
                "echo $LOCAL_ONLY > {}",
                out_path.to_string_lossy()
            ))
            .unwrap();

        assert_eq!(fs::read_to_string(out_path).unwrap(), "from-shell\n");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_export_does_not_mutate_process_environment() {
        let mut shell = Shell::new(false).unwrap();
        let key = format!("WINUXSH_NO_GLOBAL_{}", std::process::id());
        assert!(std::env::var(&key).is_err());

        shell
            .execute_command(&format!("export {}=from-shell", key))
            .unwrap();

        assert_eq!(
            shell.env_vars.get(&key),
            Some(&ArrayValue::String("from-shell".to_string()))
        );
        assert!(std::env::var(&key).is_err());
    }

    #[test]
    fn test_command_substitution_builtin_pwd_uses_shell_path() {
        let mut shell = Shell::new(false).unwrap();
        let output = shell.execute_substitution_command("pwd");
        assert_eq!(output.trim(), shell.current_dir.display().to_string());
    }

    #[test]
    fn test_command_substitution_or_uses_shell_execution() {
        let mut shell = Shell::new(false).unwrap();
        let output = shell.execute_substitution_command("notexistcmd || echo fallback");
        assert_eq!(output.trim(), "fallback");
    }

    #[test]
    fn test_command_substitution_sequence_uses_shell_execution() {
        let mut shell = Shell::new(false).unwrap();
        let output = shell.execute_substitution_command("echo one; echo two");
        assert_eq!(output.trim(), "one\ntwo");
    }

    #[test]
    fn test_command_substitution_preserves_side_effects() {
        let mut shell = Shell::new(false).unwrap();
        let _ = shell.execute_substitution_command("set SUBST_VAR=1");
        assert_eq!(
            shell.env_vars.get("SUBST_VAR"),
            Some(&ArrayValue::String("1".to_string()))
        );
    }

    #[test]
    fn test_command_substitution_failure_returns_empty_string() {
        let mut shell = Shell::new(false).unwrap();
        let output = shell.execute_substitution_command("notexistcmd");
        assert_eq!(output, "");
    }

    #[test]
    fn test_command_substitution_pipeline_captures_last_stdout() {
        let mut shell = Shell::new(false).unwrap();
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let cmd_exe = format!("{system_root}\\System32\\cmd.exe");
        let findstr_exe = format!("{system_root}\\System32\\findstr.exe");
        let command = format!("{cmd_exe} /C echo hello | {findstr_exe} h");
        let output = shell.execute_substitution_command(&command);
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn test_builtin_stdout_to_stderr_file() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir =
            std::env::temp_dir().join(format!("winuxsh_stdout_to_stderr_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let err_path = test_dir.join("err.txt");
        let cmd = format!("echo redirected 1>&2 2> {}", err_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let err = fs::read_to_string(err_path).unwrap();
        assert_eq!(err, "redirected\n");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_which_pipe_cd_changes_to_command_directory() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute_command("which cmd | cd").unwrap();
        let cwd = shell.current_dir.to_string_lossy().to_ascii_lowercase();
        assert!(cwd.contains("\\system32"));
    }

    #[test]
    fn test_background_external_command_registers_job() {
        let mut shell = Shell::new(false).unwrap();
        shell.execute_command("cmd /C exit &").unwrap();

        assert_eq!(shell.last_exit_code, 0);
        assert_eq!(shell.job_manager.job_count(), 1);
        assert_eq!(shell.job_manager.list_jobs()[0].command, "cmd /C exit");
    }
}
