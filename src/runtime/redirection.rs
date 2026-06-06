use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::tokenizer::CommandInfo;

pub fn has_redirection(cmd: &CommandInfo) -> bool {
    cmd.stdin_redir.is_some()
        || cmd.stdout_redir.is_some()
        || cmd.stderr_redir.is_some()
        || cmd.stderr_to_stdout
        || cmd.stdout_to_stderr
}

pub fn resolve_path(cwd: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

pub fn open_input(cwd: &Path, path: &str) -> Result<File> {
    Ok(File::open(resolve_path(cwd, path))?)
}

pub fn open_output(cwd: &Path, path: &str, append: bool) -> Result<File> {
    let path = resolve_path(cwd, path);
    let file = if append {
        OpenOptions::new().append(true).create(true).open(path)?
    } else {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?
    };

    Ok(file)
}

pub fn write_output(cwd: &Path, path: &str, append: bool, content: &[u8]) -> Result<()> {
    let mut file = open_output(cwd, path, append)?;
    file.write_all(content)?;
    Ok(())
}

pub fn stdio_output_handles(cwd: &Path, cmd: &CommandInfo) -> Result<(Option<File>, Option<File>)> {
    let mut stdout_handle = match cmd.stdout_redir.as_deref() {
        Some(path) => Some(open_output(cwd, path, cmd.stdout_append)?),
        None => None,
    };
    let mut stderr_handle = match cmd.stderr_redir.as_deref() {
        Some(path) => Some(open_output(cwd, path, cmd.stderr_append)?),
        None => None,
    };

    if cmd.stderr_to_stdout {
        if let Some(ref out_file) = stdout_handle {
            stderr_handle = Some(out_file.try_clone()?);
        }
    }

    if cmd.stdout_to_stderr {
        if let Some(ref err_file) = stderr_handle {
            stdout_handle = Some(err_file.try_clone()?);
        }
    }

    Ok((stdout_handle, stderr_handle))
}
