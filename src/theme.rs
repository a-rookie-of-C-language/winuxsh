// Theme plugin system for WinSH MVP6
// This demonstrates best practices for plugin development

use crate::error::Result;
use crate::plugin::Plugin;
use crate::shell::Shell;

/// Theme structure
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub prompt_user: String,
    pub prompt_host: String,
    pub prompt_dir: String,
    pub prompt_symbol: String,
}

impl Theme {
    /// Get default theme
    pub fn default_theme() -> Self {
        Theme {
            name: "default".to_string(),
            prompt_user: "\x1b[1;32m".to_string(), // Green
            prompt_host: "\x1b[1;36m".to_string(), // Cyan
            prompt_dir: "\x1b[1;34m".to_string(),  // Blue
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get dark theme
    pub fn dark_theme() -> Self {
        Theme {
            name: "dark".to_string(),
            prompt_user: "\x1b[1;38m".to_string(), // Bright white
            prompt_host: "\x1b[1;37m".to_string(), // White
            prompt_dir: "\x1b[1;35m".to_string(),  // Magenta
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get light theme
    pub fn light_theme() -> Self {
        Theme {
            name: "light".to_string(),
            prompt_user: "\x1b[1;34m".to_string(), // Blue
            prompt_host: "\x1b[1;32m".to_string(), // Green
            prompt_dir: "\x1b[1;36m".to_string(),  // Cyan
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get colorful theme
    pub fn colorful_theme() -> Self {
        Theme {
            name: "colorful".to_string(),
            prompt_user: "\x1b[1;95m".to_string(), // Bright magenta
            prompt_host: "\x1b[1;93m".to_string(), // Bright yellow
            prompt_dir: "\x1b[1;96m".to_string(),  // Bright cyan
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get minimal theme
    pub fn minimal_theme() -> Self {
        Theme {
            name: "minimal".to_string(),
            prompt_user: "".to_string(),
            prompt_host: "".to_string(),
            prompt_dir: "".to_string(),
            prompt_symbol: "".to_string(),
        }
    }

    /// Get cyberpunk theme
    pub fn cyberpunk_theme() -> Self {
        Theme {
            name: "cyberpunk".to_string(),
            prompt_user: "\x1b[1;35m".to_string(), // Magenta
            prompt_host: "\x1b[1;95m".to_string(), // Bright magenta
            prompt_dir: "\x1b[1;36m".to_string(),  // Cyan
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get ocean theme
    pub fn ocean_theme() -> Self {
        Theme {
            name: "ocean".to_string(),
            prompt_user: "\x1b[1;34m".to_string(), // Blue
            prompt_host: "\x1b[1;36m".to_string(), // Cyan
            prompt_dir: "\x1b[1;94m".to_string(),  // Bright blue
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get forest theme
    pub fn forest_theme() -> Self {
        Theme {
            name: "forest".to_string(),
            prompt_user: "\x1b[1;32m".to_string(), // Green
            prompt_host: "\x1b[1;92m".to_string(), // Bright green
            prompt_dir: "\x1b[1;33m".to_string(),  // Yellow
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }

    /// Get theme by name
    pub fn get_by_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_theme()),
            "dark" => Some(Self::dark_theme()),
            "light" => Some(Self::light_theme()),
            "colorful" => Some(Self::colorful_theme()),
            "minimal" => Some(Self::minimal_theme()),
            "cyberpunk" => Some(Self::cyberpunk_theme()),
            "ocean" => Some(Self::ocean_theme()),
            "forest" => Some(Self::forest_theme()),
            _ => None,
        }
    }

    /// Get all available themes
    pub fn all_themes() -> Vec<String> {
        vec![
            "default".to_string(),
            "dark".to_string(),
            "light".to_string(),
            "colorful".to_string(),
            "minimal".to_string(),
            "cyberpunk".to_string(),
            "ocean".to_string(),
            "forest".to_string(),
        ]
    }

    /// Generate colored prompt
    pub fn generate_prompt(&self, user: &str, host: &str, dir: &str, symbol: &str) -> String {
        format!(
            "{}{}{}@{}{} {}{}{}{}",
            self.prompt_user,
            user,
            self.prompt_symbol,
            self.prompt_host,
            host,
            self.prompt_dir,
            dir,
            self.prompt_symbol,
            symbol
        )
    }
}

impl Plugin for Theme {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool> {
        if args.is_empty() {
            // Show current theme
            println!("Current theme: {}", self.name);
            return Ok(true);
        }

        match args[0].as_str() {
            "list" => {
                println!("Available themes:");
                for theme in Theme::all_themes() {
                    println!("  {}", theme);
                }
                Ok(true)
            }
            "set" => {
                if args.len() < 2 {
                    println!("Usage: theme set <theme_name>");
                    return Ok(true);
                }

                let theme_name = &args[1];
                if let Some(theme) = Theme::get_by_name(theme_name) {
                    // Update shell theme directly
                    shell.theme_plugin = ThemePlugin::Theme(theme);
                    println!("Theme changed to: {}", theme_name);
                    Ok(true)
                } else {
                    println!("Unknown theme: {}", theme_name);
                    println!("Use 'theme list' to see available themes");
                    Ok(true)
                }
            }
            "help" => {
                println!("Theme plugin - Manage shell color themes");
                println!();
                println!("Commands:");
                println!("  theme list       List all available themes");
                println!("  theme set <name> Change current theme");
                println!("  theme help       Show this help message");
                Ok(true)
            }
            _ => {
                println!("Unknown command: {}", args[0]);
                println!("Use 'theme help' to see available commands");
                Ok(true)
            }
        }
    }

    fn description(&self) -> &str {
        "Theme plugin - Manage shell color themes"
    }
}

/// Theme plugin wrapper
#[derive(Debug, Clone)]
pub enum ThemePlugin {
    Theme(Theme),
}

impl ThemePlugin {
    pub fn new() -> Self {
        ThemePlugin::Theme(Theme::default_theme())
    }
}

impl Plugin for ThemePlugin {
    fn name(&self) -> &str {
        match self {
            ThemePlugin::Theme(theme) => theme.name(),
        }
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool> {
        match self {
            ThemePlugin::Theme(theme) => theme.execute(args, shell),
        }
    }

    fn description(&self) -> &str {
        match self {
            ThemePlugin::Theme(theme) => theme.description(),
        }
    }
}
