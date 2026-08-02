//! Binary-level tests for the non-interactive REPL command surface.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
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

#[test]
fn command_mode_can_source_user_winshrc_explicitly() {
    let temp = unique_temp_dir("winuxsh-command-mode-source-winshrc");
    let home = temp.join("home");
    let start = temp.join("start");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "").unwrap();
    std::fs::write(
        home.join(".winshrc"),
        "export WINUXSH_EXPLICIT_SOURCE_RC=loaded\n",
    )
    .unwrap();

    let output = run_winuxsh(
        &[
            "-c",
            "source ~/.winshrc; echo rc:$WINUXSH_EXPLICIT_SOURCE_RC",
        ],
        &start,
        &home,
    );

    assert_success(&output, "command mode explicit source ~/.winshrc");
    assert_eq!(stdout_text(&output).trim(), "rc:loaded");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn repl_command_cat_expands_tilde_paths_through_normal_command_resolution() {
    let temp = unique_temp_dir("winuxsh-repl-command-cat-tilde");
    let home = temp.join("home");
    let start = temp.join("start");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "").unwrap();
    std::fs::write(
        home.join(".winshrc"),
        "export WINUXSH_TILDE_CAT_RC=loaded\n",
    )
    .unwrap();

    let output = run_winuxsh(&["-C", "cat ~/.winshrc"], &start, &home);

    assert_success(&output, "repl command cat tilde expansion");
    assert_eq!(
        stdout_text(&output).trim(),
        "export WINUXSH_TILDE_CAT_RC=loaded"
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn repl_command_primary_rc_tilde_uses_windows_home_when_home_env_is_empty() {
    if !cfg!(windows) {
        return;
    }

    let temp = unique_temp_dir("winuxsh-repl-command-primary-tilde-empty-home");
    let home = temp.join("home");
    let start = temp.join("start");
    let bin = temp.join("bin");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "[winuxcmd]\nenabled = false\n").unwrap();
    std::fs::write(home.join(".winuxshrc"), "# primary marker\n").unwrap();
    std::fs::write(
        bin.join("cat.cmd"),
        "@echo off\r\nset \"arg=%~1\"\r\necho arg=%arg%\r\nif \"%arg:~0,3%\"==\"/c/\" exit /b 12\r\nset \"fsarg=%arg:/=\\%\"\r\ntype \"%fsarg%\"\r\n",
    )
    .unwrap();

    let old_path = std::env::var_os("PATH");
    let mut paths = vec![bin.clone()];
    if let Some(old_path) = old_path {
        paths.extend(std::env::split_paths(&old_path));
    }
    let output = Command::new(winuxsh_binary())
        .args(["-C", "cat ~/.winuxshrc"])
        .current_dir(&start)
        .env("HOME", "")
        .env("USERPROFILE", &home)
        .env("ZDOTDIR", &home)
        .env("WINUXSH_CONFIG", home.join(".winshrc.toml"))
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap_or_else(|err| panic!("failed to run winuxsh primary tilde test: {err}"));

    assert_success(&output, "repl command primary rc external cat tilde");
    let stdout = stdout_text(&output);
    assert!(stdout.contains("arg="), "stdout was {stdout:?}");
    assert!(
        !stdout.contains("arg=/c/"),
        "external command received slash-drive path: {stdout:?}"
    );
    assert!(stdout.contains("# primary marker"), "stdout was {stdout:?}");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn command_mode_compound_commands_keep_home_paths_native() {
    if !cfg!(windows) {
        return;
    }

    let temp = unique_temp_dir("winuxsh-command-mode-native-home-paths");
    let home = temp.join("home");
    let start = temp.join("start");
    let bin = temp.join("bin");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "[winuxcmd]\nenabled = false\n").unwrap();
    std::fs::write(home.join(".winuxshrc"), "# primary marker\n").unwrap();
    std::fs::write(
        bin.join("cat.cmd"),
        "@echo off\r\nset \"arg=%~1\"\r\necho arg=%arg%\r\nif \"%arg:~0,3%\"==\"/c/\" exit /b 12\r\nset \"fsarg=%arg:/=\\%\"\r\ntype \"%fsarg%\"\r\n",
    )
    .unwrap();

    let old_path = std::env::var_os("PATH");
    let mut paths = vec![bin.clone()];
    if let Some(old_path) = old_path {
        paths.extend(std::env::split_paths(&old_path));
    }
    let output = Command::new(winuxsh_binary())
        .args([
            "-c",
            "cd ~; echo PWD=$PWD; pwd; cat ~/.winuxshrc >/dev/null && echo catrc:ok",
        ])
        .current_dir(&start)
        .env("HOME", "")
        .env("USERPROFILE", &home)
        .env("ZDOTDIR", &home)
        .env("WINUXSH_CONFIG", home.join(".winshrc.toml"))
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap_or_else(|err| panic!("failed to run winuxsh command mode native home test: {err}"));

    assert_success(&output, "command mode compound native home paths");
    let stdout = stdout_text(&output);
    assert!(stdout.contains("catrc:ok"), "stdout was {stdout:?}");
    assert!(
        !stdout.contains("/c/"),
        "command mode leaked slash-drive paths: {stdout:?}"
    );
    assert!(stdout.contains("PWD="), "stdout was {stdout:?}");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn repl_command_file_commands_expand_tilde_paths_through_normal_command_resolution() {
    let temp = unique_temp_dir("winuxsh-repl-command-file-builtins-tilde");
    let home = temp.join("home");
    let start = temp.join("start");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "").unwrap();
    std::fs::write(home.join(".winshrc"), "").unwrap();

    let output = run_winuxsh(
        &[
            "-C",
            "mkdir -p ~/builtins/empty; touch ~/builtins/source.txt; cp ~/builtins/source.txt ~/builtins/copy.txt; rm ~/builtins/source.txt; rmdir ~/builtins/empty",
        ],
        &start,
        &home,
    );

    assert_success(&output, "repl command file command tilde expansion");
    assert!(home.join("builtins").join("copy.txt").is_file());
    assert!(!home.join("builtins").join("source.txt").exists());
    assert!(!home.join("builtins").join("empty").exists());
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn repl_command_file_command_prefers_path_over_winuxsh_native_helpers() {
    if !cfg!(windows) {
        return;
    }

    let temp = unique_temp_dir("winuxsh-repl-command-path-cat");
    let home = temp.join("home");
    let start = temp.join("start");
    let bin = temp.join("bin");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&start).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(home.join(".winshrc.toml"), "[winuxcmd]\nenabled = false\n").unwrap();
    std::fs::write(home.join(".winshrc"), "").unwrap();
    std::fs::write(bin.join("cat.cmd"), "@echo off\r\necho external-cat %*\r\n").unwrap();

    let output =
        run_winuxsh_with_extra_path(&["-C", "cat --definitely-external"], &start, &home, &bin);

    assert_success(&output, "repl command path cat");
    assert_eq!(
        stdout_text(&output).trim(),
        "external-cat --definitely-external"
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn gitstatus_daemon_returns_repo_snapshot_over_persistent_stdio() {
    let temp = unique_temp_dir("winuxsh-gitstatus-daemon");
    let repo = temp.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    for args in [
        &["init"][..],
        &["config", "user.email", "test@winuxsh"],
        &["config", "user.name", "Winuxsh Test"],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    std::fs::write(repo.join("new.txt"), "daemon\n").unwrap();

    let mut child = Command::new(winuxsh_binary())
        .arg("--gitstatus-daemon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            "{{\"id\":1,\"cwd\":{}}}",
            serde_json::to_string(&repo.to_string_lossy()).unwrap()
        )
        .unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_success(&output, "gitstatus daemon");
    let stdout = stdout_text(&output);
    assert!(stdout.contains(r#""id":1"#), "{stdout}");
    assert!(stdout.contains(r#""untracked":1"#), "{stdout}");
    assert!(stdout.contains(r#""dirty":true"#), "{stdout}");
    let _ = std::fs::remove_dir_all(temp);
}

fn run_winuxsh(args: &[&str], start: &Path, home: &Path) -> Output {
    run_winuxsh_command(args, start, home, None)
}

fn run_winuxsh_with_extra_path(
    args: &[&str],
    start: &Path,
    home: &Path,
    extra_path: &Path,
) -> Output {
    run_winuxsh_command(args, start, home, Some(extra_path))
}

fn run_winuxsh_command(
    args: &[&str],
    start: &Path,
    home: &Path,
    extra_path: Option<&Path>,
) -> Output {
    let mut command = Command::new(winuxsh_binary());
    command
        .args(args)
        .current_dir(start)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ZDOTDIR", home)
        .env("WINUXSH_CONFIG", home.join(".winshrc.toml"))
        .env_remove("WINUXSH_REPL_COMMAND_RC")
        .env_remove("WINUXSH_REPL_PRECMD_RAN")
        .env_remove("WINUXSH_REPL_PREEXEC_RAN");

    if let Some(extra_path) = extra_path {
        let old_path = std::env::var_os("PATH");
        let mut paths = vec![extra_path.to_path_buf()];
        if let Some(old_path) = old_path {
            paths.extend(std::env::split_paths(&old_path));
        }
        command.env("PATH", std::env::join_paths(paths).unwrap());
    }

    command
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
