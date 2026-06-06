// WinSH MVP6 - Array Support and Internationalization
//
// MVP6 Features:
// - Array support (definition, access, expansion)
// - Internationalization (English only)
// - Enhanced config file support (terminal styling)
// - Plugin system support
// - Modular architecture following Rust best practices

use anyhow::Result;

// Win32 API for Ctrl+C handling
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[cfg(windows)]
static CURRENT_CHILD_PID: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
static CTRL_C_RECEIVED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
use windows_sys::Win32::Foundation::BOOL;
#[cfg(windows)]
use windows_sys::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_C_EVENT};

#[cfg(windows)]
unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT => {
            // Ctrl+C received
            CTRL_C_RECEIVED.store(true, Ordering::SeqCst);

            // If there's a child process running, try to terminate it
            let current_child_pid = CURRENT_CHILD_PID.load(Ordering::SeqCst);
            if current_child_pid != 0 {
                // Terminate the child process only
                use windows_sys::Win32::System::Threading::{
                    OpenProcess, TerminateProcess, PROCESS_TERMINATE,
                };
                let handle = OpenProcess(PROCESS_TERMINATE, 0, current_child_pid);
                if !handle.is_null() {
                    TerminateProcess(handle, 1);
                }
                return 1; // Signal handled
            }

            // No child process, let the default handler run
            0
        }
        _ => 0, // Let default handlers run for other signals
    }
}
#[cfg(windows)]
pub fn setup_ctrl_c_handler() {
    unsafe {
        if SetConsoleCtrlHandler(Some(ctrl_handler), 1) == 0 {
            eprintln!("Warning: Failed to set Ctrl+C handler");
        } else {
            log::debug!("Ctrl+C handler installed successfully");
        }
    }
}

#[cfg(windows)]
pub fn set_current_child_pid(pid: u32) {
    CURRENT_CHILD_PID.store(pid, Ordering::SeqCst);
}

#[cfg(windows)]
pub fn clear_current_child_pid() {
    CURRENT_CHILD_PID.store(0, Ordering::SeqCst);
}

#[cfg(windows)]
pub fn is_ctrl_c_received() -> bool {
    CTRL_C_RECEIVED.swap(false, Ordering::SeqCst)
}

#[cfg(not(windows))]
pub fn setup_ctrl_c_handler() {}
#[cfg(not(windows))]
pub fn set_current_child_pid(_: u32) {}
#[cfg(not(windows))]
pub fn clear_current_child_pid() {}
#[cfg(not(windows))]
pub fn is_ctrl_c_received() -> bool {
    false
}

use colored::Colorize;
use reedline::Signal;
use std::env;
use std::path::PathBuf;

mod array;
#[path = "runtime/ast_adapter.rs"]
mod ast_adapter;
mod builtins;
#[path = "builtins/array.rs"]
mod builtins_array;
#[path = "builtins/jobs.rs"]
mod builtins_jobs;
#[path = "builtins/meta.rs"]
mod builtins_meta;
#[path = "runtime/capture.rs"]
mod capture;
#[path = "runtime/command_execution.rs"]
mod command_execution;
#[path = "runtime/command_lookup.rs"]
mod command_lookup;
mod command_router;
mod completion;
mod config;
mod error;
mod executor;
#[path = "runtime/expansion.rs"]
mod expansion;
mod job;
mod oh_my_winuxsh;
#[path = "runtime/pipeline.rs"]
mod pipeline;
mod plugin;
#[path = "runtime/prompt.rs"]
mod prompt;
#[path = "runtime/redirection.rs"]
mod redirection;
mod script;
#[path = "script/blocks.rs"]
mod script_blocks;
#[path = "script/expansion.rs"]
mod script_expansion;
#[path = "script/utils.rs"]
mod script_utils;
mod shell;
#[path = "shell/completion.rs"]
mod shell_completion;
#[path = "shell/config.rs"]
mod shell_config;
#[path = "shell/init.rs"]
mod shell_init;
mod theme;
mod tokenizer;
#[path = "winuxcmd/ffi.rs"]
mod winuxcmd_ffi;
#[path = "winuxcmd/locator.rs"]
mod winuxcmd_locator;

use shell::Shell;

fn print_usage() {
    println!("WinSH usage:");
    println!("  winuxsh -c \"command\"");
    println!("  winuxsh script.sh [args...]");
    println!("  winuxsh --help | -h");
    println!("  winuxsh --version");
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialize logging (default to error level only)
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Error)
        .parse_env("RUST_LOG")
        .init();

    // Setup Ctrl+C handler
    setup_ctrl_c_handler();

    // Initialize WinuxCmd FFI
    if let Err(e) = initialize_winuxcmd() {
        eprintln!("Warning: Failed to initialize WinuxCmd: {}", e);
    }

    // Parse command line arguments

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "-c" => {
                if args.len() > 2 {
                    let mut shell = Shell::new(true)?;
                    if let Err(e) = shell.save_history(&args[2]) {
                        eprintln!("{} Failed to save history: {}", "Warning:".yellow(), e);
                    }
                    shell.execute_command(&args[2])?;
                } else {
                    eprintln!("{} -c requires an argument", "Error:".red());
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
            }
            "--version" => {
                println!(
                    "{}",
                    "WinSH MVP6 - Array Support and Internationalization version 0.6.0".green()
                );
            }
            _ => {
                // Check if it's a script file
                let script_path = PathBuf::from(&args[1]);
                if script_path.exists() {
                    let mut shell = Shell::new(true)?;
                    shell.run_script_file(&script_path, &args[2..])?;
                } else {
                    eprintln!("{} {}", "Unknown argument:".red(), args[1]);
                    print_usage();
                    std::process::exit(1);
                }
            }
        }
        return Ok(());
    }

    let mut shell = Shell::new(true)?;
    shell.run_repl()?;

    Ok(())
}

// Add this to shell module temporarily
impl Shell {
    pub fn run_repl(&mut self) -> Result<()> {
        println!(
            "{}",
            "WinSH MVP6 - Array Support and Internationalization".green()
        );
        println!("Type 'help' for available commands");
        println!();

        loop {
            let prompt = self.get_prompt();

            match self.line_editor.read_line(&prompt) {
                Ok(Signal::Success(buffer)) => {
                    let line = buffer.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Err(e) = self.save_history(line) {
                        eprintln!("{} Failed to save history: {}", "Warning:".yellow(), e);
                    }

                    // Execute command
                    if let Err(e) = self.execute_command(line) {
                        eprintln!("{} {}", "Error:".red(), e);
                    }

                    // Update completion state with current directory after command execution
                    self.update_completion_state(line);
                }
                Ok(Signal::CtrlD) => {
                    println!();
                    println!("Goodbye!");
                    break;
                }
                Ok(Signal::CtrlC) => {
                    println!();
                    continue;
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red(), e);
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Initialize WinuxCmd daemon (FFI disabled, always succeeds)
fn initialize_winuxcmd() -> anyhow::Result<()> {
    Ok(())
}
