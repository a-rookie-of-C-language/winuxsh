// Oh-My-Winuxsh Plugin System
// Similar to oh-my-zsh, provides theme and plugin management

use crate::error::Result;
use crate::plugin::Plugin;
use crate::shell::Shell;
use crate::theme::{Theme, ThemePlugin};

#[derive(Debug)]
pub struct OhMyWinuxsh;

impl Plugin for OhMyWinuxsh {
    fn name(&self) -> &str {
        "oh-my-winuxsh"
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool> {
        if args.is_empty() {
            self.show_help();
            return Ok(true);
        }

        match args[0].as_str() {
            "version" => {
                println!("Oh-My-Winuxsh v1.0.0");
                println!("WinSH MVP6 Plugin System");
            }
            "list-themes" => {
                println!("Available themes:");
                for theme in Theme::all_themes() {
                    println!("  {}", theme);
                }
            }
            "set-theme" => {
                if args.len() < 2 {
                    println!("Usage: oh-my-winuxsh set-theme <theme_name>");
                    return Ok(true);
                }

                let theme_name = &args[1];
                if let Some(theme) = Theme::get_by_name(theme_name) {
                    shell.theme_plugin = ThemePlugin::Theme(theme);
                    println!("Theme changed to: {}", theme_name);
                    println!("Restart shell to see changes");
                } else {
                    println!("Unknown theme: {}", theme_name);
                }
            }
            "current-theme" => {
                let ThemePlugin::Theme(ref theme) = shell.theme_plugin;
                println!("Current theme: {}", theme.name);
            }
            "help" => {
                self.show_help();
            }
            _ => {
                println!("Unknown command: {}", args[0]);
                println!("Use 'oh-my-winuxsh help' to see available commands");
            }
        }

        Ok(true)
    }

    fn description(&self) -> &str {
        "Oh-My-Winuxsh - Theme and plugin management system"
    }
}

impl OhMyWinuxsh {
    fn show_help(&self) {
        println!("Oh-My-Winuxsh - Theme and Plugin Management");
        println!();
        println!("Commands:");
        println!("  oh-my-winuxsh version         Show version information");
        println!("  oh-my-winuxsh list-themes    List all available themes");
        println!("  oh-my-winuxsh set-theme <name>  Change current theme");
        println!("  oh-my-winuxsh current-theme  Show current theme");
        println!("  oh-my-winuxsh help           Show this help message");
        println!();
        println!("Quick Start:");
        println!("  theme list                    List themes");
        println!("  theme set <name>              Set theme");
        println!();
        println!("Available themes:");
        for theme in Theme::all_themes() {
            println!("  {}", theme);
        }
    }
}
