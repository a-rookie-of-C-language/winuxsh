use colored::Colorize;

use crate::plugin::Plugin;
use crate::shell::Shell;

impl Shell {
    pub(crate) fn handle_plugin_command(&self, args: &[String]) {
        if args.is_empty() {
            println!("Plugin commands: list, load");
            return;
        }

        match args[0].as_str() {
            "list" => {
                println!("{}", "Loaded plugins:".cyan());
                for plugin in self.plugins.list_plugins() {
                    println!("  - {}: {}", plugin.name, plugin.description);
                }
                if self.plugins.plugin_count() == 0 {
                    println!("  (No plugins loaded)");
                }
            }
            "load" => {
                if args.len() > 1 {
                    println!("Plugin '{}' not found (not implemented yet)", args[1]);
                }
            }
            _ => {
                println!("Plugin commands: list, load");
            }
        }
    }

    pub(crate) fn print_help(&self) {
        println!("{}", "WinSH MVP6 - Available commands:".green());
        println!();
        println!("{}", "Built-in commands:".cyan());
        println!("  cd [dir]       - Change directory");
        println!("  pwd            - Print current directory");
        println!("  echo [text]    - Print text (supports env vars)");
        println!("  set VAR=VALUE  - Set environment variable");
        println!("  export VAR=VALUE - Set environment variable");
        println!("  unset VAR      - Remove environment variable");
        println!("  env            - Display all environment variables");
        println!("  source [file] [args...] - Execute script in current shell");
        println!("  . [file] [args...]      - Alias for source");
        println!("  exit           - Exit shell");
        println!("  quit           - Exit shell");
        println!("  clear          - Clear screen");
        println!("  cls            - Clear screen");
        println!("  alias [name=value] - Display or set alias");
        println!("  unalias [name]  - Remove alias");
        println!("  help           - Display help information");
        println!("  history        - Display command history");
        println!("  jobs           - List background jobs");
        println!("  fg [job_id]    - Bring job to foreground");
        println!("  bg [job_id]    - Resume stopped job in background");
        println!();
        println!("{}", "Array support:".cyan());
        println!("  array define name elem1 elem2 ... - Define array");
        println!("  array get name index            - Get array element");
        println!("  array len name                  - Get array length");
        println!("  array list                      - List all arrays");
        println!();
        println!("{}", "Plugin system:".cyan());
        println!("  plugin list   - List loaded plugins");
        println!("  plugin load   - Load plugin (not implemented yet)");
        println!();
        println!("{}", "Theme system:".cyan());
        println!("  theme list              - List all available themes");
        println!("  theme set <name>        - Set a theme");
        println!("  theme current           - Show current theme");
        println!("  theme preview <name>    - Preview a theme");
        println!();
        println!("{}", "Oh-My-Winuxsh:".cyan());
        println!("  oh-my-winuxsh              - Show oh-my-winuxsh help");
        println!("  oh-my-winuxsh version       - Show version information");
        println!("  oh-my-winuxsh list-themes  - List all available themes");
        println!("  oh-my-winuxsh set-theme <name> - Change current theme");
        println!("  oh-my-winuxsh current-theme - Show current theme");
        println!();
        println!("{}", "FFI (WinuxCmd):".cyan());
        println!("  ffi_test [cmd] [args] - Test WinuxCmd FFI execution");
        println!("  ffi_version            - Show WinuxCmd version");
        println!("  ffi_commands           - List all available commands");
        println!();
        println!("{}", "Available themes:".cyan());
        println!("  default, dark, light, colorful, minimal, cyberpunk, ocean, forest");
    }

    pub(crate) fn print_history(&self) {
        if let Ok(history) = std::fs::read_to_string(&self.history_path) {
            let lines: Vec<String> = history
                .lines()
                .map(|l| {
                    l.trim_matches(|c: char| {
                        c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace()
                    })
                    .to_string()
                })
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();

            println!("{}", "Command History:".cyan());
            for (i, line) in lines.iter().enumerate() {
                println!("  {}  {}", i + 1, line);
            }
        } else {
            println!("{} No history available", "Warning:".yellow());
        }
    }

    pub(crate) fn handle_theme_command(&mut self, args: &[String]) {
        let theme_plugin = self.theme_plugin.clone();
        let result = theme_plugin.execute(args, self);
        if let Err(e) = result {
            eprintln!("{} {}", "Theme error:".red(), e);
        }
    }

    pub(crate) fn handle_ffi_test(&mut self, args: &[String]) {
        use crate::winuxcmd_ffi::WinuxCmdFFI;

        println!("{}", "WinuxCmd FFI Test".cyan());
        println!("{}", "================".cyan());

        if let Err(e) = WinuxCmdFFI::init() {
            eprintln!("{} {}", "FFI initialization failed:".red(), e);
            return;
        }

        if !WinuxCmdFFI::is_initialized() {
            eprintln!(
                "{} {} Initialization failed",
                "FFI not available:".yellow(),
                "".red()
            );
            return;
        }

        let command = if !args.is_empty() {
            args[0].clone()
        } else {
            "pwd".to_string()
        };

        let args_slice: Vec<String> = if args.len() > 1 {
            args[1..].to_vec()
        } else {
            vec![]
        };

        println!("Executing: {} {:?}", command, args_slice);
        println!();

        match WinuxCmdFFI::execute(&command, &args_slice) {
            Ok(response) => {
                if !response.stdout.is_empty() {
                    let stdout_str = String::from_utf8_lossy(&response.stdout);
                    print!("{}", stdout_str);
                }
                if !response.stderr.is_empty() {
                    let stderr_str = String::from_utf8_lossy(&response.stderr);
                    eprint!("{} {}", "Error:".red(), stderr_str);
                }
                println!("Exit code: {}", response.exit_code);
            }
            Err(e) => {
                eprintln!("{} {}", "FFI error:".red(), e);
            }
        }
    }

    pub(crate) fn handle_ffi_version(&self) {
        use crate::winuxcmd_ffi::WinuxCmdFFI;

        if let Err(e) = WinuxCmdFFI::init() {
            eprintln!("{} {}", "FFI initialization failed:".red(), e);
            return;
        }

        match WinuxCmdFFI::get_version() {
            Ok(version) => println!("WinuxCmd version: {}", version),
            Err(e) => eprintln!("{} {}", "Version error:".red(), e),
        }
    }

    pub(crate) fn handle_ffi_commands(&self) {
        use crate::winuxcmd_ffi::WinuxCmdFFI;

        if let Err(e) = WinuxCmdFFI::init() {
            eprintln!("{} {}", "FFI initialization failed:".red(), e);
            return;
        }

        match WinuxCmdFFI::get_all_commands() {
            Ok(commands) => {
                println!("{}", "Available WinuxCmd commands:".cyan());
                for command in commands {
                    println!("  {}", command);
                }
            }
            Err(e) => eprintln!("{} {}", "Commands error:".red(), e),
        }
    }
}
