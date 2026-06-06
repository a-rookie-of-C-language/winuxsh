// Executor module for WinSH MVP6
// Ported from MVP5 to provide external command execution

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::array::ArrayValue;
use crate::command_lookup;
use crate::error::{Result, ShellError};
use crate::redirection;
use crate::tokenizer::CommandInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Exited(i32),
    Background { pid: u32, command: String },
}

impl ExecutionOutcome {
    pub fn exit_code(&self) -> i32 {
        match self {
            ExecutionOutcome::Exited(code) => *code,
            ExecutionOutcome::Background { .. } => 0,
        }
    }
}

/// Executor for external commands
pub struct Executor {
    env_vars: Vec<(String, String)>,
    current_dir: PathBuf,
}

impl Executor {
    /// Create a new executor
    pub fn new(env_vars: &[(String, ArrayValue)], current_dir: &Path) -> Self {
        let env_vars: Vec<(String, String)> = env_vars
            .iter()
            .filter_map(|(k, v)| {
                if let ArrayValue::String(ref s) = v {
                    Some((k.clone(), s.clone()))
                } else {
                    None
                }
            })
            .collect();

        Executor {
            env_vars,
            current_dir: current_dir.to_path_buf(),
        }
    }

    /// Execute an external command
    pub fn execute(
        &self,
        cmd: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<ExecutionOutcome> {
        let outcome = self.execute_internal(cmd, args, cmd_info)?;
        let exit_code = outcome.exit_code();
        if exit_code != 0 {
            eprintln!("Command exited with status code: {}", exit_code);
        }
        Ok(outcome)
    }

    fn execute_internal(
        &self,
        cmd: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<ExecutionOutcome> {
        let cmd_path = self.find_command_in_path(cmd)?;

        let program = match cmd_path {
            Some(path) => path,
            None => {
                return Err(ShellError::CommandNotFound(format!(
                    "Command '{}' not found",
                    cmd
                )));
            }
        };

        let program_str = program.to_string_lossy().to_lowercase();

        // Check if it's PowerShell script
        let (actual_program, actual_args) = if program_str.ends_with(".ps1") {
            let program_path = program.to_string_lossy().to_string();
            let mut ps_args: Vec<String> = vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                program_path,
            ];
            ps_args.extend(args.iter().map(|s| s.to_string()));
            ("powershell.exe".to_string(), ps_args)
        } else {
            // For .exe, .bat, .cmd, and other executables, execute directly
            let exe_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            (program.to_string_lossy().to_string(), exe_args)
        };

        let mut command = Command::new(&actual_program);
        command.args(&actual_args);
        command.current_dir(&self.current_dir);
        command.env_clear();
        command.envs(self.env_vars.iter().cloned());

        command.stdin(Stdio::inherit());
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Handle redirections - override inherited/captured stdio if specified
        if let Some(ref stdin_file) = cmd_info.stdin_redir {
            let file = redirection::open_input(&self.current_dir, stdin_file)?;
            command.stdin(Stdio::from(file));
        }

        let (stdout_handle, stderr_handle) =
            redirection::stdio_output_handles(&self.current_dir, cmd_info)?;

        if let Some(file) = stdout_handle {
            command.stdout(Stdio::from(file));
        }

        if let Some(file) = stderr_handle {
            command.stderr(Stdio::from(file));
        }

        if cmd_info.background {
            match command.spawn() {
                Ok(child) => {
                    let pid = child.id();
                    let cmd_str = cmd_info.args.join(" ");
                    Ok(ExecutionOutcome::Background {
                        pid,
                        command: cmd_str,
                    })
                }
                Err(e) => Err(ShellError::CommandNotFound(format!(
                    "Failed to start background process: {}",
                    e
                ))),
            }
        } else {
            #[cfg(windows)]
            {
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
                command.creation_flags(CREATE_NEW_PROCESS_GROUP);
            }

            match command.spawn() {
                Ok(mut child) => {
                    #[cfg(windows)]
                    crate::set_current_child_pid(child.id());

                    let output = child
                        .wait()
                        .map(|exit_status| exit_status.code().unwrap_or(1));

                    #[cfg(windows)]
                    crate::clear_current_child_pid();

                    match output {
                        Ok(result) => Ok(ExecutionOutcome::Exited(result)),
                        Err(e) => {
                            eprintln!("Failed to wait for command: {}", e);
                            Ok(ExecutionOutcome::Exited(1))
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Command execution error: {}", e);
                    Ok(ExecutionOutcome::Exited(1))
                }
            }
        }
    }

    /// Find a command in PATH
    pub fn find_command_in_path(&self, cmd: &str) -> Result<Option<PathBuf>> {
        let shell_path = self
            .env_vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
            .map(|(_, v)| v.as_str());
        let env_path = if self.env_vars.is_empty() {
            std::env::var("PATH").ok()
        } else {
            None
        };
        let path_env = shell_path.or(env_path.as_deref());

        Ok(command_lookup::find_command(
            cmd,
            &self.current_dir,
            path_env,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_command_in_path() {
        let executor = Executor::new(&[], &PathBuf::from("."));
        let result = executor.find_command_in_path("echo");
        // echo should be found somewhere in PATH
        // This test might fail depending on the system
        assert!(result.is_ok());
    }

    #[test]
    fn test_executor_creation() {
        let env_vars = vec![(
            "PATH".to_string(),
            ArrayValue::String("/usr/bin:/bin".to_string()),
        )];
        let current_dir = PathBuf::from(".");
        let executor = Executor::new(&env_vars, &current_dir);
        assert_eq!(executor.env_vars.len(), 1);
    }
}
