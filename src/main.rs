//! winuxsh entry point
//!
//! Usage:
//!   winuxsh                  → interactive REPL
//!   winuxsh -c "command"     → execute one command, print exit code, exit
//!   winuxsh -C "command"     → execute one REPL-style command, then exit
//!   winuxsh script.sh        → execute a script file
//!   winuxsh --help | -h      → usage
//!   winuxsh --version        → version (winuxsh / rubash / winuxcmd)
//!   winuxsh setup            → re-run the interactive prompt/plugin wizard
//!   winuxsh --zsh-compat-report      → scan zsh config and print report
//!   winuxsh --zsh-compat-report-json → scan zsh config and print JSON report
//!   winuxsh --zsh-compat-import-plan → print a reviewable .winshrc.toml patch
//!   winuxsh --zsh-compat-import-apply → write the import patch with a backup
//!   winuxsh --zsh-compat-import-status → inspect import block and backups
//!   winuxsh --zsh-compat-import-rollback-plan → print restore command
//!   winuxsh --zsh-compat-doctor → summarize zsh compatibility health
//!   winuxsh plugin list [--json] → list official Winuxsh plugins
//!   winuxsh plugin info <name> [--json] → inspect one official plugin
//!   winuxsh plugin search [query] [--json] → discover official plugins
//!   winuxsh plugin themes [--json] → list user and bundle themes
//!   winuxsh plugin bundle status [--json] → inspect official bundle install state
//!   winuxsh plugin doctor [--json] → diagnose plugin configuration health
//!   winuxsh plugin review <name> [--json] → review plugin permissions
//!   winuxsh plugin update oh-my-winuxsh --from <path> → install a bundle release
//!   winuxsh plugin update oh-my-winuxsh --github-release latest → download/install bundle
//!   winuxsh plugin rollback oh-my-winuxsh → roll back to the previous bundle
//!   winuxsh plugin plan enable <name> [--json] → preview plugin TOML
//!   winuxsh plugin install <name> → install an official plugin
//!   winuxsh plugin uninstall <name> → uninstall an official plugin
//!   winuxsh plugin enable <name> → write managed plugin TOML
//!   winuxsh --zsh-native-packs → list legacy zsh migration pack mappings
//!   winuxsh --zsh-native-packs-json → list legacy zsh migration mappings as JSON
//!   winuxsh --zsh-profile-plan <profile> → print a native zsh profile TOML plan
//!   winuxsh --completion-probe "line" [cursor] → print REPL completions
//!   winuxsh --install-wt-profile → add/update the Windows Terminal profile
//!   winuxsh --self-update → download and run the latest installer
//!   self-update / update-winuxsh → REPL commands for Winuxsh self-update

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

mod self_update;
const OFFICIAL_PLUGIN_BUNDLE_REPO: &str = "unixwin/oh-my-winuxsh";
const PLUGIN_BUNDLE_DOWNLOAD_CACHE: &str = "winuxsh-plugin-bundles";

fn main() -> ExitCode {
    // Initialize logging (only error level by default)
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Error)
        .parse_env("RUST_LOG")
        .init();

    // Install Ctrl+C handler (best-effort)
    winuxsh_runtime::ctrl_c::install();

    let args: Vec<String> = std::env::args().collect();

    if let Err(e) = run(&args) {
        if is_broken_pipe_error(&e) {
            return ExitCode::from(1);
        }
        eprintln!("winuxsh: {}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run(args: &[String]) -> anyhow::Result<()> {
    if args.len() < 2 {
        return if winuxsh_runtime::terminal::stdio_is_interactive() {
            run_repl()
        } else {
            run_stdin_script()
        };
    }

    let first = &args[1];
    match first.as_str() {
        "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        "--version" | "-V" => {
            print_version();
            Ok(())
        }
        "--gitstatus-daemon" => winuxsh_runtime::git_status::run_daemon_stdio(),
        "--zsh-compat-report" => {
            print_zsh_compat_report(false)?;
            Ok(())
        }
        "--zsh-compat-report-json" => {
            print_zsh_compat_report(true)?;
            Ok(())
        }
        "--zsh-compat-import-plan" => {
            print_zsh_compat_import_plan()?;
            Ok(())
        }
        "--zsh-compat-import-apply" => {
            apply_zsh_compat_import_plan()?;
            Ok(())
        }
        "--zsh-compat-import-status" => {
            print_zsh_compat_import_status()?;
            Ok(())
        }
        "--zsh-compat-import-rollback-plan" => {
            print_zsh_compat_import_rollback_plan()?;
            Ok(())
        }
        "--zsh-compat-doctor" => {
            print_zsh_compat_doctor()?;
            Ok(())
        }
        "--zsh-native-packs" => {
            print_zsh_native_packs(false)?;
            Ok(())
        }
        "--zsh-native-packs-json" => {
            print_zsh_native_packs(true)?;
            Ok(())
        }
        "--zsh-profile-plan" => {
            print_zsh_profile_plan(args)?;
            Ok(())
        }
        "--completion-probe" => {
            print_completion_probe(args)?;
            Ok(())
        }
        "--install-wt-profile" => {
            install_windows_terminal_profile(args)?;
            Ok(())
        }
        "--self-update" => self_update::run(&args[2..]),
        "setup" | "configure" => winuxsh_runtime::setup_wizard::rerun_wizard(),
        "plugin" => run_plugin_command(args),
        "-C" | "--repl-command" => run_repl_command(args),
        "-c" => {
            if args.len() < 3 {
                anyhow::bail!("-c requires an argument");
            }
            let mut shell = winuxsh_runtime::Shell::new()?;
            shell.executor.inherit_process_stdin();
            shell.enable_process_stdin_pipeline_bridge();
            if let Some(command_name) = args.get(3) {
                shell.executor.set_env("__RUBASH_SCRIPT_NAME", command_name);
                shell.executor.set_positional_params(args[4..].to_vec());
            }
            let code = shell.execute_script(&args[2])?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        _ => {
            // Treat as a script file to execute
            let script = script_arg_to_host_path(first);
            if !script.exists() {
                anyhow::bail!("unknown argument '{}' (not a script file)", first);
            }
            let mut shell = winuxsh_runtime::Shell::new()?;
            shell.executor.set_env("__RUBASH_SCRIPT_NAME", first);
            shell.executor.inherit_process_stdin();
            shell.enable_process_stdin_pipeline_bridge();
            shell.executor.set_positional_params(args[2..].to_vec());
            let content = std::fs::read_to_string(&script)?;
            let code = shell.execute_script(&content)?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
    }
}

fn script_arg_to_host_path(value: &str) -> PathBuf {
    if cfg!(windows) {
        let normalized = value.replace('\\', "/");
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2
            && bytes[0] == b'/'
            && bytes[1].is_ascii_alphabetic()
            && (bytes.len() == 2 || bytes.get(2) == Some(&b'/'))
        {
            let drive = (bytes[1] as char).to_ascii_uppercase();
            let rest = if normalized.len() == 2 {
                "/"
            } else {
                &normalized[2..]
            };
            return PathBuf::from(format!("{drive}:{rest}"));
        }
    }

    PathBuf::from(value)
}

fn run_repl() -> anyhow::Result<()> {
    self_update::maybe_print_update_hint();
    let mut shell = winuxsh_runtime::Shell::new()?;
    winuxsh_runtime::repl::run_repl(&mut shell)
}

fn run_repl_command(args: &[String]) -> anyhow::Result<()> {
    if args.len() < 3 {
        anyhow::bail!("{} requires an argument", args[1]);
    }
    let mut shell = winuxsh_runtime::Shell::new()?;
    shell.executor.inherit_process_stdin();
    shell.enable_process_stdin_pipeline_bridge();
    if let Some(command_name) = args.get(3) {
        shell.executor.set_env("__RUBASH_SCRIPT_NAME", command_name);
        shell.executor.set_positional_params(args[4..].to_vec());
    }
    shell.run_startup_rc();
    shell.run_precmd_hooks();
    let code = shell.execute_interactive_line(&args[2])?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn run_stdin_script() -> anyhow::Result<()> {
    let mut shell = winuxsh_runtime::Shell::new_for_stdin_script()?;
    shell.executor.inherit_process_stdin();
    let mut line = String::new();
    let mut pending = Vec::new();

    loop {
        line.clear();
        match read_unbuffered_line(&mut line)? {
            0 => {
                if !pending.is_empty() {
                    let code = shell.execute_script(&pending.join("\n"))?;
                    if code != 0 {
                        std::process::exit(code);
                    }
                }
                break;
            }
            _ => {}
        }

        let line = line.trim_end_matches(['\r', '\n']);
        if pending.is_empty() && line.trim().is_empty() {
            continue;
        }
        pending.push(line.to_string());
        let script = pending.join("\n");
        if !winuxsh_runtime::repl::is_script_input_complete(&script) {
            continue;
        }

        let code = shell.execute_script(&script)?;
        if code != 0 {
            std::process::exit(code);
        }
        pending.clear();
    }

    Ok(())
}

fn read_unbuffered_line(output: &mut String) -> std::io::Result<usize> {
    let mut stdin = std::io::stdin().lock();
    let mut bytes = [0_u8; 1];
    let mut read = 0;

    loop {
        match stdin.read(&mut bytes)? {
            0 => break,
            count => {
                read += count;
                output.push(bytes[0] as char);
                if bytes[0] == b'\n' {
                    break;
                }
            }
        }
    }

    Ok(read)
}

fn print_usage() {
    println!(
        "Winuxsh {} \u{2014} a bash-compatible shell that feels at home on Windows.",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage:  winuxsh [option]");
    println!("        winuxsh -c <cmd>         Run a command then exit");
    println!("        winuxsh -C <cmd>         Run one REPL-style command then exit");
    println!("        winuxsh setup           Re-run prompt/plugin setup");
    println!("        winuxsh <script> [args]   Run a script file");
    println!();
    println!("Options:");
    println!("  -h, --help                Show this help");
    println!("  -V, --version             Version and component info");
    println!("  -c <command>              Execute a command ad-hoc");
    println!("  -C, --repl-command <cmd>  Execute one non-interactive REPL command");
    println!();
    println!("  --install-wt-profile      Add/update the Windows Terminal profile");
    println!("      --set-default         Also set Winuxsh as the WT default profile");
    println!("      --quiet               Suppress non-error profile output");
    println!("  --self-update             Download and run the latest release installer");
    println!("      --check               Only report the latest release");
    println!("      --dry-run             Download installer without running it");
    println!("  self-update               REPL command: update Winuxsh and exit this shell");
    println!("  update-winuxsh            Alias for self-update");
    println!();
    println!("  plugin list [--json]      List official Winuxsh plugins");
    println!("  plugin info <name> [--json]  Inspect one official Winuxsh plugin");
    println!("  plugin search [query] [--json]  Discover official plugins");
    println!("  plugin themes [--json]    List user and bundle themes");
    println!("  plugin bundle status [--json]  Inspect official bundle install state");
    println!("  plugin update oh-my-winuxsh --from <path>");
    println!("      [--checksum <sha>|--checksum-file <path>] [--json]");
    println!("  plugin update oh-my-winuxsh --github-release latest|vX.Y.Z [--json]");
    println!("                            Install bundle release");
    println!("  plugin rollback oh-my-winuxsh [--json]  Roll back bundle release");
    println!("  plugin plan enable <name> [--json]  Preview managed plugin TOML");
    println!("  plugin plan disable <name> [--json] Preview managed plugin TOML");
    println!("  plugin install <name>     Install official plugin from active bundle");
    println!("  plugin uninstall <name>   Uninstall official plugin from active bundle");
    println!("  plugin enable <name>      Write managed plugin TOML");
    println!("  plugin disable <name>     Write managed plugin TOML");
    println!();
    println!("  --zsh-compat-report       Scan ~/.zshrc, show safe-import report");
    println!("  --zsh-compat-report-json  Same, as JSON");
    println!("  --zsh-compat-import-plan  Generate a .winshrc.toml import patch");
    println!("  --zsh-compat-import-apply Apply the patch (with backup)");
    println!("  --zsh-compat-import-status Inspect import block and backup");
    println!("  --zsh-compat-import-rollback-plan  Show restore command");
    println!("  --zsh-compat-doctor       Legacy zsh migration health summary");
    println!();
    println!("  --zsh-native-packs        Legacy: list zsh migration pack mappings");
    println!("  --zsh-native-packs-json   Legacy: same, as JSON");
    println!("  --zsh-profile-plan <profile>  Print TOML for a profile");
    println!();
    println!("  --completion-probe <line> [cursor]  Debug: print completion candidates");
    println!();
    println!("Configuration: ~/.winshrc.toml for settings, ~/.winshrc for REPL shell code");
}

fn run_plugin_command(args: &[String]) -> anyhow::Result<()> {
    let Some(subcommand) = args.get(2) else {
        print_plugin_usage();
        return Ok(());
    };

    match subcommand.as_str() {
        "-h" | "--help" => {
            print_plugin_usage();
            Ok(())
        }
        "list" => {
            let json = parse_plugin_json_flag(&args[3..])?;
            if json {
                println!("{}", winuxsh_runtime::plugins::plugin_packs_json()?);
            } else {
                println!("{}", winuxsh_runtime::plugins::plugin_packs_text());
            }
            Ok(())
        }
        "search" => run_plugin_search_command(&args[3..]),
        "themes" => run_plugin_themes_command(&args[3..]),
        "info" => {
            let Some(name) = args.get(3) else {
                anyhow::bail!("plugin info requires a plugin name");
            };
            let json = parse_plugin_json_flag(&args[4..])?;
            if json {
                match winuxsh_runtime::plugins::plugin_pack_json(name)? {
                    Some(output) => println!("{}", output),
                    None => anyhow::bail!("unknown plugin '{}'", name),
                }
            } else {
                match winuxsh_runtime::plugins::plugin_pack_text(name) {
                    Some(output) => println!("{}", output),
                    None => anyhow::bail!("unknown plugin '{}'", name),
                }
            }
            Ok(())
        }
        "bundle" => run_plugin_bundle_command(&args[3..]),
        "doctor" => run_plugin_doctor_command(&args[3..]),
        "review" => run_plugin_review_command(&args[3..]),
        "update" => run_plugin_update_command(&args[3..]),
        "rollback" => run_plugin_rollback_command(&args[3..]),
        "plan" => run_plugin_plan_command(&args[3..]),
        "install" => run_plugin_install_command(args),
        "uninstall" => run_plugin_uninstall_command(args),
        "enable" => {
            run_plugin_apply_command(args, winuxsh_runtime::plugins::PluginConfigAction::Enable)
        }
        "disable" => {
            run_plugin_apply_command(args, winuxsh_runtime::plugins::PluginConfigAction::Disable)
        }
        unknown => anyhow::bail!("unknown plugin subcommand '{}'", unknown),
    }
}

fn run_plugin_doctor_command(args: &[String]) -> anyhow::Result<()> {
    let json = parse_plugin_json_flag(args)?;
    let config = winuxsh_runtime::config::load();
    let report = winuxsh_runtime::plugins::plugin_doctor_report(&config.plugins, &config.zsh);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", winuxsh_runtime::plugins::plugin_doctor_text(&report));
    }
    Ok(())
}

fn run_plugin_review_command(args: &[String]) -> anyhow::Result<()> {
    let Some(name) = args.get(0) else {
        anyhow::bail!("plugin review requires a plugin name");
    };
    let json = parse_plugin_json_flag(&args[1..])?;
    let config = winuxsh_runtime::config::load();
    let review =
        winuxsh_runtime::plugins::plugin_permission_review(name, &config.plugins, &config.zsh)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&review)?);
    } else {
        println!(
            "{}",
            winuxsh_runtime::plugins::plugin_permission_review_text(&review)
        );
    }
    Ok(())
}

fn run_plugin_search_command(args: &[String]) -> anyhow::Result<()> {
    let (query, json) = parse_plugin_search_args(args)?;
    if json {
        println!(
            "{}",
            winuxsh_runtime::plugins::plugin_search_json(query.as_deref())?
        );
    } else {
        println!(
            "{}",
            winuxsh_runtime::plugins::plugin_search_text(query.as_deref())
        );
    }
    Ok(())
}

fn run_plugin_themes_command(args: &[String]) -> anyhow::Result<()> {
    let json = parse_plugin_json_flag(args)?;
    if json {
        println!("{}", winuxsh_runtime::plugins::plugin_theme_catalog_json()?);
    } else {
        println!("{}", winuxsh_runtime::plugins::plugin_theme_catalog_text());
    }
    Ok(())
}

fn run_plugin_bundle_command(args: &[String]) -> anyhow::Result<()> {
    let Some(subcommand) = args.get(0) else {
        anyhow::bail!("plugin bundle requires a subcommand: status");
    };

    match subcommand.as_str() {
        "status" => {
            let json = parse_plugin_json_flag(&args[1..])?;
            if json {
                println!("{}", winuxsh_runtime::plugins::plugin_bundle_status_json()?);
            } else {
                println!("{}", winuxsh_runtime::plugins::plugin_bundle_status_text());
            }
            Ok(())
        }
        unknown => anyhow::bail!("unknown plugin bundle subcommand '{}'", unknown),
    }
}

fn run_plugin_update_command(args: &[String]) -> anyhow::Result<()> {
    let Some(bundle) = args.get(0) else {
        anyhow::bail!("plugin update requires a bundle name");
    };
    let options = parse_plugin_update_options(&args[1..])?;
    let checksum = match (options.checksum, options.checksum_file) {
        (Some(_), Some(_)) => anyhow::bail!("use only one of --checksum or --checksum-file"),
        (Some(checksum), None) => Some(checksum),
        (None, Some(path)) => Some(read_checksum_file(&path)?),
        (None, None) => None,
    };
    let github_release = options.github_release;
    let source_path = options.source_path;
    let (source_path, checksum, downloaded) = match (source_path, github_release) {
        (Some(_), Some(_)) => anyhow::bail!("use only one of --from or --github-release"),
        (Some(path), None) => (path, checksum, None),
        (None, Some(release)) => {
            if checksum.is_some() {
                anyhow::bail!(
                    "--github-release downloads and verifies the release .sha256; do not pass --checksum or --checksum-file"
                );
            }
            let downloaded = download_plugin_bundle_github_release(bundle, &release)?;
            let checksum = Some(downloaded.checksum.clone());
            (downloaded.archive_path.clone(), checksum, Some(downloaded))
        }
        (None, None) => anyhow::bail!(
            "plugin update requires --from <bundle-dir-or-zip> or --github-release latest|vX.Y.Z"
        ),
    };
    let summary = winuxsh_runtime::plugins::apply_plugin_bundle_update_from_path(
        bundle,
        &source_path,
        checksum.as_deref(),
    )?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        if let Some(downloaded) = downloaded {
            println!(
                "Downloaded GitHub release {} from {}",
                downloaded.tag, OFFICIAL_PLUGIN_BUNDLE_REPO
            );
            println!("Downloaded archive: {}", downloaded.archive_path.display());
            println!(
                "Downloaded checksum: {}",
                downloaded.checksum_path.display()
            );
        }
        println!("Updated bundle '{}' to {}", summary.bundle, summary.version);
        println!("Installed path: {}", summary.installed_path.display());
        if let Some(previous_path) = summary.previous_path {
            println!("Previous path: {}", previous_path.display());
        }
        if let Some(checksum) = summary.checksum_sha256 {
            println!("SHA-256: {}", checksum);
        }
        println!("Lock file: {}", summary.lock_path.display());
    }
    Ok(())
}
fn run_plugin_rollback_command(args: &[String]) -> anyhow::Result<()> {
    let Some(bundle) = args.get(0) else {
        anyhow::bail!("plugin rollback requires a bundle name");
    };
    let json = parse_plugin_json_flag(&args[1..])?;
    let summary = winuxsh_runtime::plugins::apply_plugin_bundle_rollback(bundle)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "Rolled back bundle '{}' to {}",
            summary.bundle, summary.version
        );
        println!("Active path: {}", summary.active_path.display());
        if let Some(previous_path) = summary.previous_path {
            println!("Previous path: {}", previous_path.display());
        }
        println!("Lock file: {}", summary.lock_path.display());
    }
    Ok(())
}
#[derive(Default)]
struct PluginUpdateOptions {
    source_path: Option<PathBuf>,
    github_release: Option<String>,
    checksum: Option<String>,
    checksum_file: Option<PathBuf>,
    json: bool,
}
fn parse_plugin_update_options(args: &[String]) -> anyhow::Result<PluginUpdateOptions> {
    let mut options = PluginUpdateOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    anyhow::bail!("--from requires a bundle directory or zip path");
                };
                options.source_path = Some(PathBuf::from(path));
            }
            "--checksum" => {
                i += 1;
                let Some(checksum) = args.get(i) else {
                    anyhow::bail!("--checksum requires a SHA-256 value");
                };
                options.checksum = Some(checksum.clone());
            }
            "--checksum-file" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    anyhow::bail!("--checksum-file requires a path");
                };
                options.checksum_file = Some(PathBuf::from(path));
            }
            "--github-release" => {
                i += 1;
                let Some(release) = args.get(i) else {
                    anyhow::bail!("--github-release requires latest or a vX.Y.Z tag");
                };
                options.github_release = Some(release.clone());
            }
            "--json" => options.json = true,
            unknown => anyhow::bail!("unknown plugin update option '{}'", unknown),
        }
        i += 1;
    }
    Ok(options)
}
struct DownloadedPluginBundle {
    archive_path: PathBuf,
    checksum_path: PathBuf,
    checksum: String,
    tag: String,
}

fn download_plugin_bundle_github_release(
    bundle: &str,
    release: &str,
) -> anyhow::Result<DownloadedPluginBundle> {
    if bundle != winuxsh_runtime::plugins::OFFICIAL_BUNDLE_NAME {
        anyhow::bail!(
            "GitHub bundle updates are only supported for {}",
            winuxsh_runtime::plugins::OFFICIAL_BUNDLE_NAME
        );
    }
    let tag = resolve_plugin_bundle_release_tag(release)?;
    let version = tag.trim_start_matches('v');
    let asset_name = format!("{bundle}-{version}.zip");
    let checksum_name = format!("{asset_name}.sha256");
    let archive_path = self_update::download_github_release_asset(
        OFFICIAL_PLUGIN_BUNDLE_REPO,
        &tag,
        &asset_name,
        PLUGIN_BUNDLE_DOWNLOAD_CACHE,
    )?;
    let checksum_path = self_update::download_github_release_asset(
        OFFICIAL_PLUGIN_BUNDLE_REPO,
        &tag,
        &checksum_name,
        PLUGIN_BUNDLE_DOWNLOAD_CACHE,
    )?;
    let checksum = read_checksum_file(&checksum_path)?;
    Ok(DownloadedPluginBundle {
        archive_path,
        checksum_path,
        checksum,
        tag,
    })
}

fn resolve_plugin_bundle_release_tag(release: &str) -> anyhow::Result<String> {
    let release = release.trim();
    if release.eq_ignore_ascii_case("latest") {
        return self_update::resolve_latest_github_release_tag(OFFICIAL_PLUGIN_BUNDLE_REPO);
    }
    normalize_plugin_bundle_release_tag(release)
}

fn normalize_plugin_bundle_release_tag(release: &str) -> anyhow::Result<String> {
    let version = release.strip_prefix('v').unwrap_or(release);
    let parts: Vec<&str> = version.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));
    if !valid {
        anyhow::bail!("--github-release must be latest or a semver tag like v1.0.0");
    }
    Ok(format!("v{version}"))
}

fn read_checksum_file(path: &PathBuf) -> anyhow::Result<String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        anyhow::anyhow!("failed to read checksum file {}: {}", path.display(), err)
    })?;
    let checksum = text
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("checksum file {} is empty", path.display()))?;
    Ok(checksum.to_string())
}
fn run_plugin_plan_command(args: &[String]) -> anyhow::Result<()> {
    let Some(action_raw) = args.get(0) else {
        anyhow::bail!("plugin plan requires an action: enable or disable");
    };
    let Some(name) = args.get(1) else {
        anyhow::bail!("plugin plan {} requires a plugin name", action_raw);
    };
    let json = parse_plugin_json_flag(&args[2..])?;
    let action = plugin_config_action_from_str(action_raw)?;
    let config_path = winuxsh_runtime::config::default_config_path();
    let plan = winuxsh_runtime::plugins::plugin_config_plan_for_path(&config_path, name, action)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("{}", plan.toml);
    }
    Ok(())
}

fn run_plugin_apply_command(
    args: &[String],
    action: winuxsh_runtime::plugins::PluginConfigAction,
) -> anyhow::Result<()> {
    let Some(name) = args.get(3) else {
        anyhow::bail!(
            "plugin {} requires a plugin name",
            plugin_config_action_name(action)
        );
    };
    reject_plugin_options(&args[4..])?;

    let config_path = winuxsh_runtime::config::default_config_path();
    let summary =
        winuxsh_runtime::plugins::apply_plugin_config_plan_to_path(&config_path, name, action)?;

    println!(
        "{} plugin '{}' in {}",
        plugin_config_action_past_tense(summary.action),
        summary.plugin,
        summary.config_path.display()
    );
    if summary.replaced_existing_block {
        println!("Replaced the previous winuxsh-managed plugin block");
    } else {
        println!("Added a new winuxsh-managed plugin block");
    }
    if let Some(backup_path) = summary.backup_path {
        println!("Backup: {}", backup_path.display());
    }
    Ok(())
}

fn run_plugin_install_command(args: &[String]) -> anyhow::Result<()> {
    let Some(name) = args.get(3) else {
        anyhow::bail!("plugin install requires a plugin name");
    };
    reject_plugin_options(&args[4..])?;

    let config_path = winuxsh_runtime::config::default_config_path();
    let summary = winuxsh_runtime::plugins::apply_plugin_config_plan_to_path(
        &config_path,
        name,
        winuxsh_runtime::plugins::PluginConfigAction::Enable,
    )?;

    println!(
        "Installed plugin '{}' in {}",
        summary.plugin,
        summary.config_path.display()
    );
    if summary.replaced_existing_block {
        println!("Replaced the previous winuxsh-managed plugin block");
    } else {
        println!("Added a new winuxsh-managed plugin block");
    }
    if let Some(backup_path) = summary.backup_path {
        println!("Backup: {}", backup_path.display());
    }
    println!("Review: winuxsh plugin review {}", summary.plugin);
    Ok(())
}

fn run_plugin_uninstall_command(args: &[String]) -> anyhow::Result<()> {
    let Some(name) = args.get(3) else {
        anyhow::bail!("plugin uninstall requires a plugin name");
    };
    reject_plugin_options(&args[4..])?;
    let config_path = winuxsh_runtime::config::default_config_path();
    let summary = winuxsh_runtime::plugins::apply_plugin_config_plan_to_path(
        &config_path,
        name,
        winuxsh_runtime::plugins::PluginConfigAction::Disable,
    )?;
    println!(
        "Uninstalled plugin '{}' in {}",
        summary.plugin,
        summary.config_path.display()
    );
    if summary.replaced_existing_block {
        println!("Replaced the previous winuxsh-managed plugin block");
    } else {
        println!("Added a new winuxsh-managed plugin block");
    }
    if let Some(backup_path) = summary.backup_path {
        println!("Backup: {}", backup_path.display());
    }
    println!("Install: winuxsh plugin install {}", summary.plugin);
    Ok(())
}
fn plugin_config_action_from_str(
    value: &str,
) -> anyhow::Result<winuxsh_runtime::plugins::PluginConfigAction> {
    match value {
        "enable" => Ok(winuxsh_runtime::plugins::PluginConfigAction::Enable),
        "disable" => Ok(winuxsh_runtime::plugins::PluginConfigAction::Disable),
        unknown => anyhow::bail!("unknown plugin plan action '{}'", unknown),
    }
}

fn plugin_config_action_name(action: winuxsh_runtime::plugins::PluginConfigAction) -> &'static str {
    match action {
        winuxsh_runtime::plugins::PluginConfigAction::Enable => "enable",
        winuxsh_runtime::plugins::PluginConfigAction::Disable => "disable",
    }
}

fn plugin_config_action_past_tense(
    action: winuxsh_runtime::plugins::PluginConfigAction,
) -> &'static str {
    match action {
        winuxsh_runtime::plugins::PluginConfigAction::Enable => "Enabled",
        winuxsh_runtime::plugins::PluginConfigAction::Disable => "Disabled",
    }
}

fn reject_plugin_options(args: &[String]) -> anyhow::Result<()> {
    for arg in args {
        anyhow::bail!("unknown plugin option '{}'", arg);
    }
    Ok(())
}

fn parse_plugin_json_flag(args: &[String]) -> anyhow::Result<bool> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            unknown => anyhow::bail!("unknown plugin option '{}'", unknown),
        }
    }
    Ok(json)
}

fn parse_plugin_search_args(args: &[String]) -> anyhow::Result<(Option<String>, bool)> {
    let mut query = None;
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value if value.starts_with("-") => {
                anyhow::bail!("unknown plugin search option {}", value)
            }
            value => {
                if query.is_some() {
                    anyhow::bail!("plugin search accepts at most one query");
                }
                query = Some(value.to_string());
            }
        }
    }
    Ok((query, json))
}

fn print_plugin_usage() {
    println!("Usage:  winuxsh plugin <command>");
    println!();
    println!("Commands:");
    println!("  list [--json]             List official Winuxsh plugins");
    println!("  info <name> [--json]      Inspect one official Winuxsh plugin");
    println!("  search [query] [--json]   Discover official plugins");
    println!("  themes [--json]           List user and bundle themes");
    println!("  bundle status [--json]    Inspect official bundle install state");
    println!("  doctor [--json]           Diagnose plugin configuration health");
    println!("  review <name> [--json]    Review plugin permissions before enabling");
    println!("  update oh-my-winuxsh --from <path>");
    println!("      [--checksum <sha>|--checksum-file <path>] [--json]");
    println!("                            Install a local bundle directory or zip");
    println!("  update oh-my-winuxsh --github-release latest|vX.Y.Z [--json]");
    println!("                            Download, verify, and install GitHub release");
    println!("  rollback oh-my-winuxsh [--json]");
    println!("                            Roll back to the previous bundle");
    println!("  plan enable <name> [--json]   Preview managed plugin TOML");
    println!("  plan disable <name> [--json]  Preview managed plugin TOML");
    println!("  install <name>           Install official plugin from active bundle");
    println!("  uninstall <name>         Uninstall official plugin from active bundle");
    println!("  enable <name>             Write managed plugin TOML");
    println!("  disable <name>            Write managed plugin TOML");
}

fn install_windows_terminal_profile(args: &[String]) -> anyhow::Result<()> {
    let mut set_default = false;
    let mut quiet = false;

    for arg in &args[2..] {
        match arg.as_str() {
            "--set-default" => set_default = true,
            "--quiet" => quiet = true,
            unknown => anyhow::bail!("unknown --install-wt-profile option '{}'", unknown),
        }
    }

    let commandline = std::env::current_exe()?;
    let icon = windows_terminal_icon_path(&commandline);
    let summary = winuxsh_runtime::windows_terminal::install_winuxsh_profile(
        &commandline,
        icon.as_deref(),
        set_default,
    )?;

    if !quiet {
        if summary.updated.is_empty() {
            println!("No Windows Terminal settings path was found.");
        } else {
            for path in summary.updated {
                println!("Updated Windows Terminal profile: {}", path.display());
            }
        }
    }

    Ok(())
}

fn windows_terminal_icon_path(commandline: &std::path::Path) -> Option<PathBuf> {
    let app_dir = commandline.parent()?;
    [
        app_dir.join("assets").join("winuxsh-icon-256.png"),
        app_dir.join("assets").join("winuxsh-icon.png"),
        app_dir.join("winuxsh-icon-256.png"),
        app_dir.join("winuxsh-icon.png"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn print_completion_probe(args: &[String]) -> anyhow::Result<()> {
    if args.len() < 3 {
        anyhow::bail!("--completion-probe requires an input line");
    }
    let line = &args[2];
    let cursor_pos = if let Some(raw) = args.get(3) {
        raw.parse::<usize>()
            .map_err(|_| anyhow::anyhow!("invalid cursor position '{}'", raw))?
    } else {
        line.len()
    };
    let mut shell = winuxsh_runtime::Shell::new()?;
    shell.run_startup_rc();
    for suggestion in shell.completion_probe(line, cursor_pos) {
        println!("{}", suggestion);
    }
    Ok(())
}

fn print_zsh_compat_import_plan() -> anyhow::Result<()> {
    let config = winuxsh_runtime::config::load();
    let options = winuxsh_runtime::zsh_compat::ZshImportOptions::for_report(&config.zsh);
    let report = winuxsh_runtime::zsh_compat::scan(&options);
    println!(
        "{}",
        winuxsh_runtime::zsh_compat::import_plan_toml(&options, &report)
    );
    Ok(())
}

fn apply_zsh_compat_import_plan() -> anyhow::Result<()> {
    let config = winuxsh_runtime::config::load();
    let options = winuxsh_runtime::zsh_compat::ZshImportOptions::for_report(&config.zsh);
    let report = winuxsh_runtime::zsh_compat::scan(&options);
    let plan = winuxsh_runtime::zsh_compat::import_plan_toml(&options, &report);
    let config_path = winuxsh_runtime::config::default_config_path();
    let summary = winuxsh_runtime::zsh_compat::apply_import_plan_to_config(&config_path, &plan)?;

    println!(
        "Wrote zsh compatibility import block to {}",
        summary.config_path.display()
    );
    if summary.replaced_existing_block {
        println!("Replaced the previous winuxsh-managed zsh import block");
    } else {
        println!("Added a new winuxsh-managed zsh import block");
    }
    if let Some(backup_path) = summary.backup_path {
        println!("Backup: {}", backup_path.display());
    }
    Ok(())
}

fn print_zsh_compat_import_status() -> anyhow::Result<()> {
    let config = winuxsh_runtime::config::load();
    let options = winuxsh_runtime::zsh_compat::ZshImportOptions::for_report(&config.zsh);
    let report = winuxsh_runtime::zsh_compat::scan(&options);
    let plan = winuxsh_runtime::zsh_compat::import_plan_toml(&options, &report);
    let config_path = winuxsh_runtime::config::default_config_path();
    let status = winuxsh_runtime::zsh_compat::inspect_import_config_status(&config_path, &plan)?;

    println!("Config: {}", status.config_path.display());
    println!("Exists: {}", yes_no(status.config_exists));
    println!(
        "Managed block: {}",
        zsh_import_block_state_label(status.block_state)
    );
    if status.toml_valid {
        println!("TOML: valid");
    } else {
        println!(
            "TOML: invalid ({})",
            status.toml_error.as_deref().unwrap_or("unknown error")
        );
    }
    println!(
        "Next apply: {}",
        zsh_import_apply_readiness_label(status.apply_readiness)
    );
    if let Some(error) = status.apply_error {
        println!("Apply detail: {}", error);
    }
    println!("Backups: {}", status.backup_paths.len());
    if let Some(path) = status.backup_paths.last() {
        println!("Latest backup: {}", path.display());
    }
    Ok(())
}

fn print_zsh_compat_import_rollback_plan() -> anyhow::Result<()> {
    let config_path = winuxsh_runtime::config::default_config_path();
    let plan = winuxsh_runtime::zsh_compat::inspect_import_rollback_plan(&config_path)?;

    println!("Config: {}", plan.config_path.display());
    println!("Backups: {}", plan.backup_paths.len());
    if let Some(path) = plan.latest_backup_path {
        println!("Latest backup: {}", path.display());
    } else {
        println!("Latest backup: none");
    }
    if let Some(command) = plan.restore_command {
        println!("Restore command:");
        println!("{}", command);
    } else {
        println!("Restore command: unavailable (no backups found)");
    }
    Ok(())
}

fn print_zsh_compat_doctor() -> anyhow::Result<()> {
    let config = winuxsh_runtime::config::load();
    let options = winuxsh_runtime::zsh_compat::ZshImportOptions::for_report(&config.zsh);
    let report = winuxsh_runtime::zsh_compat::scan(&options);
    let plan = winuxsh_runtime::zsh_compat::import_plan_toml(&options, &report);
    let config_path = winuxsh_runtime::config::default_config_path();
    let status = winuxsh_runtime::zsh_compat::inspect_import_config_status(&config_path, &plan)?;
    let rollback = winuxsh_runtime::zsh_compat::inspect_import_rollback_plan(&config_path)?;

    println!(
        "{}",
        winuxsh_runtime::zsh_compat::zsh_compat_doctor_text(&report, &status, &rollback)
    );
    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn zsh_import_block_state_label(
    state: winuxsh_runtime::zsh_compat::ZshImportBlockState,
) -> &'static str {
    match state {
        winuxsh_runtime::zsh_compat::ZshImportBlockState::Missing => "missing",
        winuxsh_runtime::zsh_compat::ZshImportBlockState::Present => "present",
        winuxsh_runtime::zsh_compat::ZshImportBlockState::Malformed => "malformed",
    }
}

fn zsh_import_apply_readiness_label(
    readiness: winuxsh_runtime::zsh_compat::ZshImportApplyReadiness,
) -> &'static str {
    match readiness {
        winuxsh_runtime::zsh_compat::ZshImportApplyReadiness::AddNewBlock => "add new block",
        winuxsh_runtime::zsh_compat::ZshImportApplyReadiness::ReplaceExistingBlock => {
            "replace existing block"
        }
        winuxsh_runtime::zsh_compat::ZshImportApplyReadiness::Blocked => "blocked",
    }
}

fn print_zsh_compat_report(json: bool) -> anyhow::Result<()> {
    let config = winuxsh_runtime::config::load();
    let options = winuxsh_runtime::zsh_compat::ZshImportOptions::for_report(&config.zsh);
    let report = winuxsh_runtime::zsh_compat::scan(&options);
    if json {
        println!("{}", report.to_json_pretty()?);
    } else {
        println!("{}", report.to_human());
    }
    Ok(())
}

fn print_zsh_native_packs(json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", winuxsh_runtime::zsh_compat::native_zsh_packs_json()?);
    } else {
        println!("{}", winuxsh_runtime::zsh_compat::native_zsh_packs_text());
    }
    Ok(())
}

fn print_zsh_profile_plan(args: &[String]) -> anyhow::Result<()> {
    let Some(profile) = args.get(2) else {
        anyhow::bail!("--zsh-profile-plan requires a profile: agent or zsh-lite");
    };
    println!(
        "{}",
        winuxsh_runtime::zsh_compat::zsh_profile_plan_toml_for_name(profile)?
    );
    Ok(())
}

fn print_version() {
    println!(
        "Winuxsh {} \u{2014} bash-compatible shell for Windows",
        env!("CARGO_PKG_VERSION")
    );
    println!("  rubash   git {}", rubash_revision());
    if let Some(v) = winuxsh_runtime::winuxcmd::version() {
        println!("  winuxcmd {}", v);
    }
}

fn rubash_revision() -> &'static str {
    option_env!("WINUXSH_RUBASH_REV").unwrap_or("master")
}

fn is_broken_pipe_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(is_broken_pipe_io_error)
            || cause.to_string().contains("os error 232")
            || cause.to_string().contains("管道正在被关闭")
    })
}

fn is_broken_pipe_io_error(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::BrokenPipe || error.raw_os_error() == Some(232)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_update_parses_github_release() {
        let args = vec![
            "--github-release".to_string(),
            "latest".to_string(),
            "--json".to_string(),
        ];
        let options = parse_plugin_update_options(&args).unwrap();

        assert_eq!(options.github_release.as_deref(), Some("latest"));
        assert!(options.json);
        assert!(options.source_path.is_none());
    }

    #[test]
    fn plugin_release_tag_normalizes_semver() {
        assert_eq!(
            normalize_plugin_bundle_release_tag("1.2.3").unwrap(),
            "v1.2.3"
        );
        assert_eq!(
            normalize_plugin_bundle_release_tag("v1.2.3").unwrap(),
            "v1.2.3"
        );
        assert!(normalize_plugin_bundle_release_tag("stable").is_err());
        assert!(normalize_plugin_bundle_release_tag("v1.2").is_err());
        assert!(normalize_plugin_bundle_release_tag("v1.2.3.4").is_err());
    }
}
