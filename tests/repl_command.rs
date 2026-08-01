//! Binary-level tests for the non-interactive REPL command surface.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn winuxsh_binary() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_BIN_EXE_winuxsh"));
    if p.exists() {
        return p;
    }
    let mut fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fallback.push("target");
    fallback.push("debug");
    fallback.push(if cfg!(windows) {
        "winuxsh.exe"
    } else {
        "winuxsh"
    });
    fallback
}

#[test]
fn repl_command_runs_startup_rc_and_lifecycle_hooks_without_banner() {
    let temp = unique_temp_dir("winuxsh-repl-command");
    let home = temp.join("home");
    let start = temp.join("start");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::write(
        home.join(".winshrc.toml"),
        r#"
[hooks]
precmd = ["export WINUXSH_REPL_PRECMD_RAN=yes"]
preexec = ["export WINUXSH_REPL_PREEXEC_RAN=yes"]
"#,
    )
    .unwrap();
    std::fs::write(
        home.join(".winshrc"),
        "export WINUXSH_REPL_COMMAND_RC=loaded\n",
    )
    .unwrap();

    let output = run_winuxsh(
        &[
            "-C",
            "echo rc:$WINUXSH_REPL_COMMAND_RC precmd:$WINUXSH_REPL_PRECMD_RAN preexec:$WINUXSH_REPL_PREEXEC_RAN",
        ],
        &start,
        &home,
    );

    assert_success(&output, "repl command");
    let stdout = stdout_text(&output);
    assert_eq!(
        stdout.trim(),
        "rc:loaded precmd:yes preexec:yes",
        "stdout was {stdout:?}"
    );
    assert!(
        !stdout.contains("Winuxsh "),
        "one-shot REPL command should not print the interactive banner"
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn command_mode_keeps_script_semantics_without_repl_startup() {
    let temp = unique_temp_dir("winuxsh-command-mode");
    let home = temp.join("home");
    let start = temp.join("start");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::write(
        home.join(".winshrc.toml"),
        r#"
[hooks]
precmd = ["export WINUXSH_REPL_PRECMD_RAN=yes"]
preexec = ["export WINUXSH_REPL_PREEXEC_RAN=yes"]
"#,
    )
    .unwrap();
    std::fs::write(
        home.join(".winshrc"),
        "export WINUXSH_REPL_COMMAND_RC=loaded\n",
    )
    .unwrap();

    let output = run_winuxsh(
        &[
            "-c",
            "echo rc:$WINUXSH_REPL_COMMAND_RC precmd:$WINUXSH_REPL_PRECMD_RAN preexec:$WINUXSH_REPL_PREEXEC_RAN",
        ],
        &start,
        &home,
    );

    assert_success(&output, "command mode");
    assert_eq!(
        stdout_text(&output).trim(),
        "rc: precmd: preexec:",
        "ordinary -c must stay on the script command path"
    );
    let _ = std::fs::remove_dir_all(temp);
}

fn run_winuxsh(args: &[&str], start: &Path, home: &Path) -> Output {
    Command::new(winuxsh_binary())
        .args(args)
        .current_dir(start)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ZDOTDIR", home)
        .env("WINUXSH_CONFIG", home.join(".winshrc.toml"))
        .env_remove("WINUXSH_REPL_COMMAND_RC")
        .env_remove("WINUXSH_REPL_PRECMD_RAN")
        .env_remove("WINUXSH_REPL_PREEXEC_RAN")
        .output()
        .unwrap_or_else(|err| panic!("failed to run winuxsh {args:?}: {err}"))
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed: status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        stdout_text(output),
        stderr_text(output)
    );
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n")
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
}
