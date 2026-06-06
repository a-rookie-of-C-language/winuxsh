use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::array::ArrayValue;
use crate::error::{Result, ShellError};
use crate::redirection;
use crate::shell::Shell;
use crate::tokenizer::CommandInfo;

impl Shell {
    /// Execute a pipeline.
    pub fn execute_pipeline(&mut self, cmds: &[CommandInfo]) -> Result<()> {
        log::debug!("execute_pipeline: {} commands", cmds.len());

        if cmds.is_empty() {
            return Ok(());
        }

        if cmds.len() == 1 {
            return self.execute_single_command(&cmds[0]);
        }

        if self.try_pipe_to_cd(&cmds[0], &cmds[1..])? {
            return Ok(());
        }

        if let Some(input) = self.try_builtin_pipeline_input(&cmds[0]) {
            return self.execute_real_pipeline_with_input(&cmds[1..], input.into_bytes());
        }

        if self.try_builtin_pipeline_side_effect(&cmds[0])? {
            return self.execute_real_pipeline_with_input(&cmds[1..], Vec::new());
        }

        self.execute_real_pipeline(cmds)
    }

    fn try_builtin_pipeline_input(&self, first: &CommandInfo) -> Option<String> {
        if first.args.is_empty() {
            return None;
        }

        match first.args[0].as_str() {
            "env" => {
                let mut output = String::new();
                for (key, value) in &self.env_vars {
                    match value {
                        ArrayValue::String(v) => {
                            output.push_str(&format!("{}={}\n", key, v));
                        }
                        ArrayValue::Array(arr) => {
                            output.push_str(&format!("{}=({})\n", key, arr.join(" ")));
                        }
                    }
                }
                Some(output)
            }
            "echo" => {
                let text = first.args[1..].join(" ");
                Some(format!("{text}\n"))
            }
            "pwd" => Some(format!("{}\n", self.current_dir.display())),
            _ => None,
        }
    }

    fn try_builtin_pipeline_side_effect(&mut self, first: &CommandInfo) -> Result<bool> {
        if first.args.is_empty() {
            return Ok(false);
        }

        match first.args[0].as_str() {
            "cd" => {
                if let Some(result) = self.handle_builtin(&first.args) {
                    result?;
                    self.last_exit_code = 0;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    fn try_pipe_to_cd(&mut self, first: &CommandInfo, rest: &[CommandInfo]) -> Result<bool> {
        if rest.len() != 1 || first.args.is_empty() || rest[0].args.is_empty() {
            return Ok(false);
        }
        if first.args[0] != "which" || rest[0].args[0] != "cd" {
            return Ok(false);
        }
        if first.args.len() < 2 {
            return Ok(false);
        }

        let target = &first.args[1];
        let cmd_path = self.find_command_in_path(target).ok_or_else(|| {
            ShellError::CommandNotFound(format!("Command '{}' not found", target))
        })?;

        println!("{}", cmd_path.display());

        let new_dir = if cmd_path.is_dir() {
            cmd_path
        } else if let Some(parent) = cmd_path.parent() {
            parent.to_path_buf()
        } else {
            return Ok(true);
        };

        self.current_dir = new_dir.canonicalize().map_err(|e| {
            ShellError::InvalidCommand(format!("cd: {} - {}", new_dir.display(), e))
        })?;
        self.last_exit_code = 0;
        Ok(true)
    }

    fn execute_real_pipeline(&mut self, cmds: &[CommandInfo]) -> Result<()> {
        if cmds.is_empty() {
            return Ok(());
        }

        let env_vars = self.pipeline_env_vars();
        let mut children: Vec<Child> = Vec::new();
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (i, cmd) in cmds.iter().enumerate() {
            let is_last = i == cmds.len() - 1;

            if cmd.args.is_empty() {
                return Err(ShellError::Parse("Empty command in pipeline".to_string()));
            }

            let cmd_name = &cmd.args[0];
            let cmd_args = &cmd.args[1..];
            let (program, stage_args) = self.resolve_pipeline_stage(cmd_name, cmd_args)?;

            let mut process = Command::new(&program);
            process.args(&stage_args);
            process.env_clear();
            process.envs(env_vars.iter().cloned());
            process.current_dir(&self.current_dir);

            if let Some(stdout) = prev_stdout.take() {
                process.stdin(Stdio::from(stdout));
            } else if let Some(ref stdin_file) = cmd.stdin_redir {
                let file = redirection::open_input(&self.current_dir, stdin_file)?;
                process.stdin(Stdio::from(file));
            } else {
                process.stdin(Stdio::inherit());
            }

            if is_last {
                if let Some(ref stdout_file) = cmd.stdout_redir {
                    let file = redirection::open_output(
                        &self.current_dir,
                        stdout_file,
                        cmd.stdout_append,
                    )?;
                    process.stdout(Stdio::from(file));
                } else {
                    process.stdout(Stdio::inherit());
                }
            } else {
                process.stdout(Stdio::piped());
            }

            if let Some(ref stderr_file) = cmd.stderr_redir {
                let file =
                    redirection::open_output(&self.current_dir, stderr_file, cmd.stderr_append)?;
                process.stderr(Stdio::from(file));
            } else {
                process.stderr(Stdio::inherit());
            }

            set_pipeline_process_group(&mut process);

            let mut child = process.spawn().map_err(|e| {
                ShellError::CommandNotFound(format!("Failed to execute '{}': {}", cmd_name, e))
            })?;

            log::debug!(
                "Spawned process '{}' (PID: {:?}) in pipeline (is_last: {})",
                cmd_name,
                child.id(),
                is_last
            );

            if !is_last {
                prev_stdout = child.stdout.take();
            }

            children.push(child);
        }

        drop(prev_stdout);
        self.wait_for_pipeline(children)
    }

    fn execute_real_pipeline_with_input(
        &mut self,
        cmds: &[CommandInfo],
        input: Vec<u8>,
    ) -> Result<()> {
        use std::io::Write;

        if cmds.is_empty() {
            self.last_exit_code = 0;
            return Ok(());
        }

        let env_vars = self.pipeline_env_vars();
        let mut children: Vec<Child> = Vec::new();
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (i, cmd) in cmds.iter().enumerate() {
            let is_last = i == cmds.len() - 1;
            if cmd.args.is_empty() {
                return Err(ShellError::Parse("Empty command in pipeline".to_string()));
            }

            let cmd_name = &cmd.args[0];
            let cmd_args = &cmd.args[1..];
            let (program, stage_args) = self.resolve_pipeline_stage(cmd_name, cmd_args)?;

            let mut process = Command::new(&program);
            process.args(&stage_args);
            process.env_clear();
            process.envs(env_vars.iter().cloned());
            process.current_dir(&self.current_dir);

            if i == 0 {
                process.stdin(Stdio::piped());
            } else if let Some(stdout) = prev_stdout.take() {
                process.stdin(Stdio::from(stdout));
            } else {
                process.stdin(Stdio::inherit());
            }

            if is_last {
                process.stdout(Stdio::inherit());
            } else {
                process.stdout(Stdio::piped());
            }
            process.stderr(Stdio::inherit());

            set_pipeline_process_group(&mut process);

            let mut child = process.spawn().map_err(|e| {
                ShellError::CommandNotFound(format!("Failed to execute '{}': {}", cmd_name, e))
            })?;

            if i == 0 {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&input);
                }
            }

            if !is_last {
                prev_stdout = child.stdout.take();
            }
            children.push(child);
        }

        self.wait_for_pipeline(children)
    }

    fn resolve_pipeline_stage(
        &self,
        cmd_name: &str,
        cmd_args: &[String],
    ) -> Result<(String, Vec<String>)> {
        let route_decision = if let Some(router) = &self.command_router {
            router.route_command(cmd_name)
        } else {
            crate::command_router::RouteDecision::ExternalCommand
        };

        match route_decision {
            crate::command_router::RouteDecision::WinuxCmdDLL(_) => {
                self.resolve_winuxcmd_pipeline_stage(cmd_name, cmd_args)
            }
            _ => {
                if let Some(cmd_path) = self.find_command_in_path(cmd_name) {
                    Ok((cmd_path.to_string_lossy().to_string(), cmd_args.to_vec()))
                } else if self.is_winuxcmd_classified(cmd_name) {
                    self.resolve_winuxcmd_pipeline_stage(cmd_name, cmd_args)
                } else {
                    Err(ShellError::CommandNotFound(format!(
                        "Command '{}' not found",
                        cmd_name
                    )))
                }
            }
        }
    }

    fn resolve_winuxcmd_pipeline_stage(
        &self,
        cmd_name: &str,
        cmd_args: &[String],
    ) -> Result<(String, Vec<String>)> {
        let winuxcmd_bin = self
            .find_command_in_path("winuxcmd")
            .or_else(|| self.find_winuxcmd_binary_path())
            .ok_or_else(|| {
                ShellError::CommandNotFound(
                    "winuxcmd executable not found for pipeline stage".to_string(),
                )
            })?;

        let mut args = Vec::with_capacity(cmd_args.len() + 1);
        args.push(cmd_name.to_string());
        args.extend(cmd_args.iter().cloned());
        Ok((winuxcmd_bin.to_string_lossy().to_string(), args))
    }

    fn pipeline_env_vars(&self) -> Vec<(String, String)> {
        self.env_vars
            .iter()
            .filter_map(|(k, v)| {
                if let ArrayValue::String(ref s) = v {
                    Some((k.clone(), s.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn wait_for_pipeline(&mut self, children: Vec<Child>) -> Result<()> {
        let mut last_exit_code = 0;
        for mut child in children {
            let status = child.wait().map_err(|e| {
                ShellError::CommandNotFound(format!("Failed to wait for process: {}", e))
            })?;
            last_exit_code = status.code().unwrap_or(1);
        }
        self.last_exit_code = last_exit_code;
        Ok(())
    }
}

#[cfg(windows)]
fn set_pipeline_process_group(process: &mut Command) {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    process.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(windows))]
fn set_pipeline_process_group(_process: &mut Command) {}
