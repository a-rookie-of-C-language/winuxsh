use std::path::PathBuf;

use crate::shell::Shell;

impl Shell {
    pub(crate) fn register_command_completion_plugin(&self) {
        if !self.config.completions.enable_command_completion {
            return;
        }

        use crate::completion::external::CommandCompletionPlugin;
        if let Ok(mut state) = self.completion_state.lock() {
            state.add_plugin(std::sync::Arc::new(CommandCompletionPlugin));
        }
    }

    pub(crate) fn register_external_completion_plugin(&self) {
        use crate::completion::external::ExternalCompletionPlugin;

        let mut dirs: Vec<PathBuf> = self
            .config
            .completions
            .completion_dirs
            .iter()
            .map(|dir| {
                if dir.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&dir[2..])
                    } else {
                        PathBuf::from(dir)
                    }
                } else {
                    PathBuf::from(dir)
                }
            })
            .collect();

        if let Some(home) = dirs::home_dir() {
            let default_dir = home.join(".winsh").join("completions");
            if !dirs.contains(&default_dir) {
                dirs.push(default_dir);
            }
        }

        let mut plugin = ExternalCompletionPlugin::new();

        for dir in &dirs {
            if !dir.exists() {
                log::debug!("External completion dir {:?} does not exist, skipping", dir);
                continue;
            }
            plugin.load_dir(dir);
        }

        plugin.enrich_descriptions_from_help();

        if plugin.definition_count() > 0 {
            if let Ok(mut state) = self.completion_state.lock() {
                state.add_plugin(std::sync::Arc::new(plugin));
            }
        }
    }

    /// Update completion state with an executed command and directory/env changes.
    pub fn update_completion_state(&self, command: &str) {
        let update = if let Ok(mut state) = self.completion_state.lock() {
            let directory_changed = state.current_dir != self.current_dir;
            state.current_dir = self.current_dir.clone();
            state.env_vars = self.env_vars.clone();
            Some((state.plugins.clone(), directory_changed))
        } else {
            None
        };

        if let Some((plugins, directory_changed)) = update {
            for plugin in plugins {
                plugin.on_command_executed(command);
                if directory_changed {
                    plugin.on_directory_changed(&self.current_dir);
                }
            }
        }
    }
}
