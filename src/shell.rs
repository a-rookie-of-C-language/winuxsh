use std::collections::HashMap;
use std::env;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use reedline::{Reedline, FileBackedHistory, DefaultCompleter, ColumnarMenu,
               ReedlineMenu, DefaultPrompt, DefaultPromptSegment, Emacs,
               KeyCode, KeyModifiers, ReedlineEvent, default_emacs_keybindings,
               MenuBuilder};
use colored::Colorize;

use crate::error::Result;
use crate::array::ArrayValue;
use crate::config::ShellConfig;
use crate::plugin::PluginManager;
use crate::job::JobManager;
use crate::tokenizer::{Tokenizer, ParsedCommand, CommandInfo};
use crate::parser::Parser;
use crate::executor::Executor;
use crate::theme::ThemePlugin;
use glob;

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
}

impl Shell {
    /// Create a new shell instance
    pub fn new(load_config: bool) -> Result<Self> {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let history_path = home_dir.join(".winsh_history");

        // Get commands from PATH for completion
        fn get_path_commands() -> Vec<String> {
            let mut commands = Vec::new();
            
            if let Ok(path_env) = std::env::var("PATH") {
                for path in env::split_paths(&path_env) {
                    if let Ok(entries) = std::fs::read_dir(path) {
                        for entry in entries.flatten() {
                            if let Ok(file_type) = entry.file_type() {
                                if file_type.is_file() {
                                    let file_name = entry.file_name().to_string_lossy().to_string();
                                    // Check if it's executable by extension
                                    let is_executable = file_name.ends_with(".exe") || 
                                                       file_name.ends_with(".bat") || 
                                                       file_name.ends_with(".cmd") ||
                                                       file_name.ends_with(".ps1");
                                    
                                    if is_executable {
                                        // Remove extension for cleaner completion
                                        let name_without_ext = if let Some(pos) = file_name.rfind('.') {
                                            file_name[..pos].to_string()
                                        } else {
                                            file_name.clone()
                                        };
                                        commands.push(name_without_ext);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            commands
        }
        
        let path_commands = get_path_commands();
        
        // Create command list for completion (built-in + PATH commands)
        let builtin_commands = vec![
            "ls".to_string(),
            "cd".to_string(),
            "pwd".to_string(),
            "echo".to_string(),
            "exit".to_string(),
            "clear".to_string(),
            "cat".to_string(),
            "grep".to_string(),
            "find".to_string(),
            "cp".to_string(),
            "mv".to_string(),
            "rm".to_string(),
            "mkdir".to_string(),
            "jobs".to_string(),
            "fg".to_string(),
            "bg".to_string(),
            "set".to_string(),
            "unset".to_string(),
            "export".to_string(),
            "env".to_string(),
            "help".to_string(),
            "history".to_string(),
            "alias".to_string(),
            "unalias".to_string(),
            "source".to_string(),
            "array".to_string(),
            "plugin".to_string(),
            "theme".to_string(),
            "oh-my-winuxsh".to_string(),
        ];
        
        let all_commands: Vec<String> = builtin_commands
            .into_iter()
            .chain(path_commands)
            .collect();
        
        // Sort and deduplicate commands
        let mut unique_commands: Vec<_> = all_commands.into_iter().collect();
        unique_commands.sort();
        unique_commands.dedup();
        
        // Create completer with all commands
        let completer = Box::new(DefaultCompleter::new_with_wordlen(unique_commands, 2));
        
        // Create completion menu (exactly like MVP4)
        let completion_menu = Box::new(
            ColumnarMenu::default()
                .with_name("completion_menu")
                .with_marker("? ")
        );

        // Setup TAB key binding for completion (exactly like MVP4)
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        let edit_mode = Box::new(Emacs::new(keybindings));

        // Create history with a graceful fallback when file access is denied.
        // This keeps tests and restricted environments from panicking.
        let history = match FileBackedHistory::with_file(1000, history_path.clone()) {
            Ok(h) => h,
            Err(e) => {
                eprintln!(
                    "{} {}",
                    "Warning:".yellow(),
                    format!("Failed to open history file, using in-memory history: {}", e)
                );
                match FileBackedHistory::new(1000) {
                    Ok(h) => h,
                    Err(fallback_err) => {
                        return Err(crate::error::ShellError::Config(format!(
                            "Failed to initialize history: {}; fallback failed: {}",
                            e, fallback_err
                        )));
                    }
                }
            }
        };

        // Create line editor with edit mode and completion (exactly like MVP4)
        let line_editor = Reedline::create()
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_edit_mode(edit_mode)
            .with_history(Box::new(history))
            .with_quick_completions(true)
            .with_partial_completions(true);

        let mut shell = Shell {
            current_dir: std::env::current_dir()?,
            aliases: HashMap::new(),
            env_vars: HashMap::new(),
            line_editor,
            history_path,
            config: ShellConfig::default(),
            plugins: PluginManager::new(),
            job_manager: JobManager::new(),
            theme_plugin: ThemePlugin::new(),
        };

        // Load default aliases
        shell.aliases.insert("ll".to_string(), "ls -la".to_string());
        shell.aliases.insert("la".to_string(), "ls -a".to_string());
        shell.aliases.insert("l".to_string(), "ls".to_string());

        // Load environment variables
        for (key, value) in std::env::vars() {
            shell.env_vars.insert(key, ArrayValue::String(value));
        }

        // Automatically add winuxcmd directory to PATH (MVP5 compatibility)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let winuxcmd_dir = exe_dir.join("winuxcmd");
                if winuxcmd_dir.exists() {
                    let winuxcmd_path = winuxcmd_dir.to_string_lossy().to_string();
                    if let Some(path_value) = shell.env_vars.iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                        .map(|(_, v)| v.clone()) {
                        // Add winuxcmd to end of PATH to avoid affecting system commands
                        let new_path = format!("{};{}", path_value, winuxcmd_path);
                        shell.env_vars.insert("PATH".to_string(), ArrayValue::String(new_path.clone()));
                        std::env::set_var("PATH", &new_path);
                    } else {
                        shell.env_vars.insert("PATH".to_string(), ArrayValue::String(winuxcmd_path.clone()));
                        std::env::set_var("PATH", &winuxcmd_path);
                    }
                }
            }
        }

        // Load configuration
        if load_config {
            if let Err(e) = shell.load_config() {
                eprintln!("{} {}", "Warning:".yellow(), format!("Failed to load config: {}", e));
            }
        }

        // Initialize plugins
        use crate::plugin::WelcomePlugin;
        use crate::oh_my_winuxsh::OhMyWinuxsh;

        // Add welcome plugin
        if let Err(e) = shell.plugins.add_plugin(Box::new(WelcomePlugin)) {
            eprintln!("{} {}", "Warning:".yellow(), format!("Failed to load welcome plugin: {}", e));
        }

        // Add oh-my-winuxsh plugin
        if let Err(e) = shell.plugins.add_plugin(Box::new(OhMyWinuxsh)) {
            eprintln!("{} {}", "Warning:".yellow(), format!("Failed to load oh-my-winuxsh plugin: {}", e));
        }

        Ok(shell)
    }

    /// Load configuration
    fn load_config(&mut self) -> Result<()> {
        use crate::config::ConfigManager;

        // Load shell config
        if let Some(config_path) = ConfigManager::find_config_file() {
            if config_path.extension().map(|e| e == "toml").unwrap_or(false) {
                println!("{} {}", "Loading shell config:".cyan(), config_path.display());
                let mut config_manager = ConfigManager::new();
                self.config = config_manager.load_config(&config_path)?;
            } else {
                println!("{} {}", "Loading config:".cyan(), config_path.display());
                self.parse_config_file(&config_path)?;
            }
        }

        Ok(())
    }

    /// Parse configuration file
    pub fn parse_config_file(&mut self, path: &Path) -> Result<()> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Execute config commands
            // TODO: Implement proper command execution
            let _ = line;
        }

        Ok(())
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str, default: &str) -> String {
        self.env_vars.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| ArrayValue::String(default.to_string()))
            .as_string()
            .unwrap_or(default)
            .to_string()
    }

    /// Get the prompt string
    pub fn get_prompt(&self) -> DefaultPrompt {
        let username = self.get_env_var("USERNAME", "user");
        let hostname = self.get_env_var("COMPUTERNAME", "localhost");
        let dir = self.current_dir.display().to_string();

        // Use theme plugin if available, otherwise use default colors
        let prompt_text = if let ThemePlugin::Theme(ref theme) = self.theme_plugin {
            theme.generate_prompt(&username, &hostname, &dir, "$ ")
        } else {
            // Default colored prompt
            format!("\x1b[1;32m{}@{}\x1b[0m \x1b[1;34m{}\x1b[0m $ ", username, hostname, dir)
        };

        DefaultPrompt::new(
            DefaultPromptSegment::Basic(prompt_text),
            DefaultPromptSegment::Empty
        )
    }

    /// Parse ANSI escape sequences from config format
    fn parse_ansi_sequence(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            chars.next();

            if c == '\\' {
                if let Some(&'x') = chars.peek() {
                    chars.next(); // consume 'x'
                    if let Some(&'1') = chars.peek() {
                        chars.next(); // consume '1'
                        if let Some(&'b') = chars.peek() {
                            chars.next(); // consume 'b'
                            // This is \x1b, add actual ANSI escape
                            result.push('\x1b');
                        } else {
                            result.push_str("\\x1");
                        }
                    } else {
                        result.push_str("\\x");
                    }
                } else {
                    result.push(c);
                }
            } else if c == '\x1b' {
                // Already an escape sequence, keep it
                result.push(c);
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Save command to history
    pub fn save_history(&mut self, command: &str) -> Result<()> {
        let clean_command = command.trim_matches(|c: char| {
            c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace()
        });

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

    /// Execute a command
    pub fn execute_command(&mut self, command: &str) -> Result<()> {
        // Tokenize the command
        let tokens = Tokenizer::tokenize(command)?;
        
        // Parse the tokens into an AST
        let parsed = Parser::parse(&tokens)?;
        
        // Execute the parsed command
        self.execute_parsed(&parsed)?;
        
        Ok(())
    }
    
    /// Execute a parsed command
    pub fn execute_parsed(&mut self, parsed: &ParsedCommand) -> Result<()> {
        match parsed {
            ParsedCommand::Single(cmd) => {
                self.execute_single_command(cmd)?;
            }
            ParsedCommand::Pipeline(cmds) => {
                self.execute_pipeline(cmds)?;
            }
            ParsedCommand::And(left, right) => {
                // Execute left command, only execute right if left succeeds
                if self.execute_parsed(left).is_ok() {
                    self.execute_parsed(right)?;
                }
            }
            ParsedCommand::Or(left, right) => {
                // Execute left command, only execute right if left fails
                if self.execute_parsed(left).is_err() {
                    self.execute_parsed(right)?;
                }
            }
            ParsedCommand::Sequence(cmds) => {
                // Execute commands in sequence
                for cmd in cmds {
                    self.execute_parsed(cmd)?;
                }
            }
        }
        Ok(())
    }
    
    /// Execute a single command
    pub fn execute_single_command(&mut self, cmd: &CommandInfo) -> Result<()> {
        // Skip empty commands
        if cmd.args.is_empty() {
            return Ok(());
        }

        // Clone the command info for modification
        let mut cmd_clone = cmd.clone();

        // Expand aliases
        let first_arg = &cmd_clone.args[0];
        if let Some(alias_cmd) = self.aliases.get(first_arg) {
            let alias_parts: Vec<String> = alias_cmd.split_whitespace().map(|s| s.to_string()).collect();
            if !alias_parts.is_empty() {
                cmd_clone.args[0] = alias_parts[0].clone();
                cmd_clone.args.splice(1..1, alias_parts[1..].iter().cloned());
            }
        }

        // Get command name
        let clean_command = cmd_clone.args[0].trim_matches(|c: char| {
            c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace()
        }).to_string();

        // Expand command substitution in arguments
        let args_with_substitution: Vec<String> = cmd_clone.args[1..]
            .iter()
            .map(|arg| self.expand_command_substitution(arg))
            .collect();

        // Expand wildcards in arguments (skip the command name)
        let expanded_args = self.expand_wildcards(&args_with_substitution);
        
        // Combine command name with expanded arguments
        let all_args: Vec<String> = vec![clean_command.clone()]
            .into_iter()
            .chain(expanded_args)
            .collect();

        // Now check if it's a built-in command with expanded arguments
        if let Some(result) = self.handle_builtin(&all_args) {
            return result;
        }

        let args: Vec<String> = all_args[1..].to_vec();

        // Convert environment variables to the format expected by Executor
        let env_vars: Vec<(String, ArrayValue)> = self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Create executor
        let executor = Executor::new(&env_vars, &self.current_dir);

        // Execute the external command
        let mut cmd_info = cmd_clone;
        cmd_info.args = all_args;
        
        match executor.execute(&clean_command, &args, &cmd_info) {
            Ok(_exit_code) => {
                // Command executed successfully
                Ok(())
            }
            Err(e) => {
                eprintln!("{} {}", "Error:".red(), e);
                Ok(())
            }
        }
    }
    
    /// Execute a pipeline
    pub fn execute_pipeline(&mut self, cmds: &[CommandInfo]) -> Result<()> {
        // Simplified pipeline execution
        // TODO: Implement proper pipeline with process pipes
        for cmd in cmds {
            self.execute_single_command(cmd)?;
        }
        Ok(())
    }

    /// Expand wildcards in arguments
    pub fn expand_wildcards(&self, args: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();
        
        for arg in args {
            if arg.contains('*') || arg.contains('?') || arg.contains('[') {
                // Expand wildcard
                if let Ok(matches) = glob::glob(arg) {
                    for entry in matches.flatten() {
                        expanded.push(entry.to_string_lossy().to_string());
                    }
                    // If no matches found, keep the original pattern
                    if expanded.is_empty() || expanded.last() != Some(arg) {
                        expanded.push(arg.clone());
                    }
                } else {
                    // Invalid pattern, keep original
                    expanded.push(arg.clone());
                }
            } else {
                // No wildcard, keep as is
                expanded.push(arg.clone());
            }
        }
        
        expanded
    }

    /// Expand command substitution $(...)
    pub fn expand_command_substitution(&mut self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&'(') = chars.peek() {
                    // Start of command substitution
                    chars.next(); // consume '('
                    let mut command = String::new();
                    let mut depth = 1;
                    
                    while let Some(&c) = chars.peek() {
                        chars.next(); // consume char
                        if c == '(' {
                            depth += 1;
                            command.push(c);
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break; // End of command substitution
                            } else {
                                command.push(c);
                            }
                        } else {
                            command.push(c);
                        }
                    }
                    
                    // Execute the command and capture output
                    let output = self.execute_and_capture(&command);
                    result.push_str(&output.trim());
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }
        
        result
    }

    /// Execute command and capture output
    fn execute_and_capture(&mut self, command: &str) -> String {
        use std::process::Command;
        
        match Command::new("cmd").args(["/C", command]).output() {
            Ok(output) => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            Err(_) => String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
