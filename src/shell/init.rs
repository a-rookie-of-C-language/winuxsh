use std::collections::HashMap;
use std::path::PathBuf;

use colored::Colorize;
use reedline::{
    default_emacs_keybindings, Emacs, FileBackedHistory, KeyCode, KeyModifiers, ListMenu,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu,
};

use crate::array::ArrayValue;
use crate::config::ShellConfig;
use crate::error::Result;
use crate::job::JobManager;
use crate::plugin::PluginManager;
use crate::shell::Shell;
use crate::theme::ThemePlugin;

impl Shell {
    /// Create a new shell instance.
    pub fn new(load_config: bool) -> Result<Self> {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let history_path = home_dir.join(".winsh_history");

        let current_dir = std::env::current_dir()?;
        let completion_state = std::sync::Arc::new(std::sync::Mutex::new(
            crate::completion::CompletionState::new(current_dir.clone()),
        ));

        let line_editor = create_line_editor(history_path.clone(), completion_state.clone())?;
        let command_router = load_command_router();

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
            last_exit_code: 0,
            command_router,
            completion_state,
        };

        shell.load_default_aliases();
        shell.load_process_environment();
        shell.add_bundled_winuxcmd_to_path();
        shell.load_startup_config(load_config);
        shell.register_startup_completion(load_config);
        shell.load_builtin_plugins();

        Ok(shell)
    }

    fn load_default_aliases(&mut self) {
        self.aliases.insert("ll".to_string(), "ls -la".to_string());
        self.aliases.insert("la".to_string(), "ls -a".to_string());
        self.aliases.insert("l".to_string(), "ls".to_string());
    }

    fn load_process_environment(&mut self) {
        for (key, value) in std::env::vars() {
            self.env_vars.insert(key, ArrayValue::String(value));
        }
    }

    fn add_bundled_winuxcmd_to_path(&mut self) {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let winuxcmd_dir = exe_dir.join("winuxcmd");
                if winuxcmd_dir.exists() {
                    let winuxcmd_path = winuxcmd_dir.to_string_lossy().to_string();
                    if let Some(path_value) = self
                        .env_vars
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                        .map(|(_, v)| v.clone())
                    {
                        let new_path = format!("{};{}", path_value, winuxcmd_path);
                        self.env_vars
                            .insert("PATH".to_string(), ArrayValue::String(new_path));
                    } else {
                        self.env_vars
                            .insert("PATH".to_string(), ArrayValue::String(winuxcmd_path));
                    }
                }
            }
        }
    }

    fn load_startup_config(&mut self, load_config: bool) {
        if !load_config {
            return;
        }

        if let Err(e) = self.load_config() {
            eprintln!("{} Failed to load config: {}", "Warning:".yellow(), e);
            return;
        }

        if let Some(router) = &mut self.command_router {
            let enable_dll = self.config.winuxcmd.enable_dll;
            router.set_enable_dll(enable_dll);
        }
    }

    fn register_startup_completion(&self, load_config: bool) {
        self.register_command_completion_plugin();

        if load_config {
            self.register_external_completion_plugin();
        }
    }

    fn load_builtin_plugins(&mut self) {
        use crate::oh_my_winuxsh::OhMyWinuxsh;
        use crate::plugin::WelcomePlugin;

        if let Err(e) = self.plugins.add_plugin(Box::new(WelcomePlugin)) {
            eprintln!(
                "{} Failed to load welcome plugin: {}",
                "Warning:".yellow(),
                e
            );
        }

        if let Err(e) = self.plugins.add_plugin(Box::new(OhMyWinuxsh)) {
            eprintln!(
                "{} Failed to load oh-my-winuxsh plugin: {}",
                "Warning:".yellow(),
                e
            );
        }
    }
}

fn create_line_editor(
    history_path: PathBuf,
    completion_state: std::sync::Arc<std::sync::Mutex<crate::completion::CompletionState>>,
) -> Result<Reedline> {
    use crate::completion::WinuxshCompleter;
    use nu_ansi_term::{Color, Style};

    let completer = Box::new(WinuxshCompleter::new(completion_state));
    let completion_menu = Box::new(
        ListMenu::default()
            .with_name("completion_menu")
            .with_only_buffer_difference(false)
            .with_marker("> ")
            .with_text_style(Style::new().fg(Color::White))
            .with_selected_text_style(Style::new().fg(Color::Black).on(Color::Fixed(39)))
            .with_match_text_style(Style::new().fg(Color::Fixed(114)).bold())
            .with_selected_match_text_style(
                Style::new().fg(Color::Black).on(Color::Fixed(39)).bold(),
            )
            .with_page_size(12),
    );

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
    let history = open_history(history_path)?;

    Ok(Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_history(Box::new(history))
        .with_quick_completions(false)
        .with_partial_completions(true))
}

fn open_history(history_path: PathBuf) -> Result<FileBackedHistory> {
    match FileBackedHistory::with_file(1000, history_path) {
        Ok(history) => Ok(history),
        Err(e) => {
            eprintln!(
                "{} Failed to open history file, using in-memory history: {}",
                "Warning:".yellow(),
                e
            );
            FileBackedHistory::new(1000).map_err(|fallback_err| {
                crate::error::ShellError::Config(format!(
                    "Failed to initialize history: {}; fallback failed: {}",
                    e, fallback_err
                ))
            })
        }
    }
}

fn load_command_router() -> Option<crate::command_router::CommandRouter> {
    match crate::command_router::load_classification() {
        Ok(classification) => Some(crate::command_router::CommandRouter::new(
            classification,
            true,
        )),
        Err(e) => {
            eprintln!(
                "{} Failed to load command classification: {}",
                "Warning:".yellow(),
                e
            );
            None
        }
    }
}
