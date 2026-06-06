use std::path::PathBuf;

use colored::Colorize;
use log::debug;
use winsh_lexer::Lexer;
use winsh_parser::Parser;

use crate::array::ArrayValue;
use crate::error::{Result, ShellError};
use crate::executor::{ExecutionOutcome, Executor};
use crate::redirection;
use crate::shell::Shell;
use crate::tokenizer::{CommandInfo, ParsedCommand};

impl Shell {
    pub fn execute_command(&mut self, command: &str) -> Result<()> {
        // Update completion state with executed command
        self.update_completion_state(command);

        let tokens = Lexer::tokenize(command).map_err(|e| ShellError::Parse(e.to_string()))?;
        let stmts = Parser::parse(tokens).map_err(|e| ShellError::Parse(e.to_string()))?;

        let parsed = crate::ast_adapter::convert_to_parsed_command(&stmts, &|name| {
            self.resolve_env_var(name)
        });
        self.execute_parsed(&parsed)?;

        Ok(())
    }

    /// Execute a parsed command.
    pub fn execute_parsed(&mut self, parsed: &ParsedCommand) -> Result<()> {
        match parsed {
            ParsedCommand::Single(cmd) => self.execute_single_command(cmd),
            ParsedCommand::Pipeline(cmds) => self.execute_pipeline(cmds),
            ParsedCommand::And(left, right) => {
                self.execute_parsed(left)?;
                if self.last_exit_code == 0 {
                    self.execute_parsed(right)?;
                }
                Ok(())
            }
            ParsedCommand::Or(left, right) => {
                self.execute_parsed(left)?;
                if self.last_exit_code != 0 {
                    self.execute_parsed(right)?;
                }
                Ok(())
            }
            ParsedCommand::Sequence(commands) => {
                for cmd in commands {
                    self.execute_parsed(cmd)?;
                }
                Ok(())
            }
        }
    }

    /// Execute a single command.
    pub fn execute_single_command(&mut self, cmd: &CommandInfo) -> Result<()> {
        if cmd.args.is_empty() {
            return Ok(());
        }

        let mut cmd_clone = cmd.clone();

        let first_arg = &cmd_clone.args[0];
        if let Some(alias_cmd) = self.aliases.get(first_arg) {
            let alias_parts: Vec<String> = alias_cmd
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if !alias_parts.is_empty() {
                cmd_clone.args[0] = alias_parts[0].clone();
                cmd_clone
                    .args
                    .splice(1..1, alias_parts[1..].iter().cloned());
            }
        }

        let clean_command = cmd_clone.args[0]
            .trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace())
            .to_string();

        let args_with_substitution: Vec<String> = cmd_clone.args[1..]
            .iter()
            .map(|arg| self.expand_command_substitution(arg))
            .collect();
        let expanded_args = self.expand_wildcards(&args_with_substitution);

        let all_args: Vec<String> = vec![clean_command.clone()]
            .into_iter()
            .chain(expanded_args)
            .collect();

        if self.try_builtin_with_redirection(&clean_command, &all_args, &cmd_clone)? {
            self.last_exit_code = 0;
            return Ok(());
        }

        if let Some(result) = self.handle_builtin(&all_args) {
            match result {
                Ok(()) => {
                    self.last_exit_code = 0;
                    return Ok(());
                }
                Err(e) => {
                    self.last_exit_code = 1;
                    if clean_command != "[" && clean_command != "test" {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                    return Ok(());
                }
            }
        }

        if let Some(router) = &self.command_router {
            let route_decision = router.route_command(&clean_command);

            match route_decision {
                crate::command_router::RouteDecision::Builtin => {}
                crate::command_router::RouteDecision::WinuxCmdDLL(_category) => {
                    let args: Vec<String> = all_args[1..].to_vec();
                    return self.execute_winuxcmd_command(&clean_command, &args, &cmd_clone);
                }
                crate::command_router::RouteDecision::ExternalCommand => {}
            }
        }

        let args: Vec<String> = all_args[1..].to_vec();
        let env_vars = self.executor_env_vars();
        let executor = Executor::new(&env_vars, &self.current_dir);

        let mut cmd_info = cmd_clone;
        cmd_info.args = all_args;

        match executor.execute(&clean_command, &args, &cmd_info) {
            Ok(outcome) => {
                self.handle_execution_outcome(outcome);
                Ok(())
            }
            Err(ShellError::CommandNotFound(_)) => {
                self.try_winuxcmd_fallback(&clean_command, &args, &cmd_info)
            }
            Err(e) => {
                self.last_exit_code = 1;
                eprintln!("{} {}", "Error:".red(), e);
                Ok(())
            }
        }
    }

    fn try_winuxcmd_fallback(
        &mut self,
        command: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<()> {
        let winuxcmd = match self.find_winuxcmd_on_path() {
            Some(p) => p,
            None => {
                self.last_exit_code = 127;
                eprintln!("{} Command '{}' not found", "Error:".red(), command);
                return Ok(());
            }
        };

        let env_vars = self.executor_env_vars();
        let executor = Executor::new(&env_vars, &self.current_dir);
        let mut winuxcmd_args = vec![command.to_string()];
        winuxcmd_args.extend(args.iter().cloned());

        match executor.execute(&winuxcmd.to_string_lossy(), &winuxcmd_args, cmd_info) {
            Ok(outcome) => {
                self.handle_execution_outcome(outcome);
                Ok(())
            }
            Err(e) => {
                self.last_exit_code = 127;
                eprintln!("{} {}", "Error:".red(), e);
                Ok(())
            }
        }
    }

    fn execute_winuxcmd_command(
        &mut self,
        command: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<()> {
        use crate::winuxcmd_ffi::WinuxCmdFFI;

        debug!("Executing via WinuxCmd DLL: {} {:?}", command, args);

        if cmd_info.stdin_redir.is_some() {
            log::debug!("Command has stdin redirection, falling back to external execution");
            return self.execute_external_command_fallback(command, args, cmd_info);
        }

        match WinuxCmdFFI::execute(command, args) {
            Ok(response) => {
                if let Some(ref stdout_file) = cmd_info.stdout_redir {
                    if !response.stdout.is_empty() {
                        redirection::write_output(
                            &self.current_dir,
                            stdout_file,
                            cmd_info.stdout_append,
                            &response.stdout,
                        )?;
                    }
                } else if !response.stdout.is_empty() {
                    let stdout_str = String::from_utf8_lossy(&response.stdout);
                    print!("{}", stdout_str);
                }

                if let Some(ref stderr_file) = cmd_info.stderr_redir {
                    if !response.stderr.is_empty() {
                        redirection::write_output(
                            &self.current_dir,
                            stderr_file,
                            cmd_info.stderr_append,
                            &response.stderr,
                        )?;
                    }
                } else if cmd_info.stderr_to_stdout {
                    if let Some(ref stdout_file) = cmd_info.stdout_redir {
                        if !response.stderr.is_empty() {
                            redirection::write_output(
                                &self.current_dir,
                                stdout_file,
                                cmd_info.stdout_append,
                                &response.stderr,
                            )?;
                        }
                    } else if !response.stderr.is_empty() {
                        let stderr_str = String::from_utf8_lossy(&response.stderr);
                        print!("{}", stderr_str);
                    }
                } else if !response.stderr.is_empty() {
                    let stderr_str = String::from_utf8_lossy(&response.stderr);
                    eprint!("{}", stderr_str);
                }

                self.last_exit_code = response.exit_code;

                if response.exit_code != 0 {
                    eprintln!("Command exited with status code: {}", response.exit_code);
                }

                Ok(())
            }
            Err(e) => {
                eprintln!("{} WinuxCmd DLL failed: {}", "Warning:".yellow(), e);
                eprintln!("Falling back to external command execution");
                self.execute_external_command_fallback(command, args, cmd_info)
            }
        }
    }

    fn execute_external_command_fallback(
        &mut self,
        command: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<()> {
        debug!("Executing via PATH: {} {:?}", command, args);

        let env_vars = self.executor_env_vars();
        let executor = Executor::new(&env_vars, &self.current_dir);

        let mut cmd_info_clone = cmd_info.clone();
        cmd_info_clone.args = vec![command.to_string()]
            .into_iter()
            .chain(args.iter().cloned())
            .collect();

        match executor.execute(command, args, &cmd_info_clone) {
            Ok(outcome) => {
                self.handle_execution_outcome(outcome);
                Ok(())
            }
            Err(e) => {
                self.last_exit_code = match e {
                    ShellError::CommandNotFound(_) => 127,
                    _ => 1,
                };
                eprintln!("{} {}", "Error:".red(), e);
                Ok(())
            }
        }
    }

    fn handle_execution_outcome(&mut self, outcome: ExecutionOutcome) {
        match outcome {
            ExecutionOutcome::Exited(code) => {
                self.last_exit_code = code;
            }
            ExecutionOutcome::Background { pid, command } => {
                let job_id = self.job_manager.add_job(command.clone(), pid);
                self.last_exit_code = 0;
                println!("Background job started: [{}] {} {}", job_id, pid, command);
            }
        }
    }

    pub(crate) fn find_winuxcmd_binary_path(&self) -> Option<PathBuf> {
        crate::winuxcmd_locator::find_bundled(&self.current_dir)
    }

    fn find_winuxcmd_on_path(&self) -> Option<PathBuf> {
        let path = self.resolve_env_var("PATH");
        crate::winuxcmd_locator::find_in_path(&self.current_dir, path.as_deref())
    }

    pub(crate) fn is_winuxcmd_classified(&self, cmd_name: &str) -> bool {
        if let Some(router) = &self.command_router {
            router.classification().is_winuxcmd_command(cmd_name)
        } else {
            false
        }
    }

    /// Find command in PATH.
    pub(crate) fn find_command_in_path(&self, cmd: &str) -> Option<PathBuf> {
        let env_vars = self.executor_env_vars();
        let executor = Executor::new(&env_vars, &self.current_dir);
        executor.find_command_in_path(cmd).unwrap_or_default()
    }

    fn try_builtin_with_redirection(
        &mut self,
        command: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<bool> {
        if !redirection::has_redirection(cmd_info) {
            return Ok(false);
        }

        match command {
            "echo" => {
                let mut output = args[1..].join(" ");
                output.push('\n');
                self.write_builtin_output(&output, cmd_info)?;
                Ok(true)
            }
            "pwd" => {
                let output = format!("{}\n", self.current_dir.display());
                self.write_builtin_output(&output, cmd_info)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn write_builtin_output(&self, output: &str, cmd_info: &CommandInfo) -> Result<()> {
        if cmd_info.stdout_to_stderr {
            if let Some(ref stderr_file) = cmd_info.stderr_redir {
                redirection::write_output(
                    &self.current_dir,
                    stderr_file,
                    cmd_info.stderr_append,
                    output.as_bytes(),
                )?;
            } else {
                eprint!("{}", output);
            }
        } else if let Some(ref stdout_file) = cmd_info.stdout_redir {
            redirection::write_output(
                &self.current_dir,
                stdout_file,
                cmd_info.stdout_append,
                output.as_bytes(),
            )?;
        } else {
            print!("{}", output);
        }

        if let Some(ref stderr_file) = cmd_info.stderr_redir {
            if !cmd_info.stdout_to_stderr {
                let _ = redirection::open_output(
                    &self.current_dir,
                    stderr_file,
                    cmd_info.stderr_append,
                )?;
            }
        }

        Ok(())
    }

    fn executor_env_vars(&self) -> Vec<(String, ArrayValue)> {
        self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}
