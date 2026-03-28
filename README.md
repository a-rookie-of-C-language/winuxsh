# WinSH - Windows Shell

A modern Unix-style command-line shell for Windows, written in Rust. WinSH provides a powerful shell experience with full compatibility with Windows commands and Unix-style tools.

## Features

### Core Functionality
- **860+ Command Completion**: Auto-discovery of commands from PATH
- **Wildcard Expansion**: Full support for `*`, `?`, `[]` patterns
- **Command Substitution**: Execute commands within commands using `$(command)`
- **Script Execution**: Run `.sh` scripts with full shell support
- **History Management**: Browse command history with arrow keys

### Advanced Features
- **Array System**: Define, access, and manipulate arrays
- **Plugin Architecture**: Extensible plugin system for custom functionality
- **Theme Management**: 8 built-in themes with color customization
- **Environment Variables**: Full support for environment variable management
- **Emacs Mode**: Powerful keybindings for efficient editing

### Built-in Commands
- `ls` - List directory contents
- `cd` - Change directory
- `pwd` - Print working directory
- `echo` - Display text
- `cat` - Display file contents
- `grep` - Search text
- `find` - Find files
- `cp` - Copy files
- `mv` - Move/rename files
- `rm` - Remove files
- `mkdir` - Create directories
- `jobs` - List background jobs
- `fg` - Bring job to foreground
- `bg` - Send job to background
- `set` - Set environment variables
- `unset` - Unset variables
- `export` - Export variables
- `env` - Display environment
- `help` - Display help
- `history` - Command history
- `alias` - Create command aliases
- `unalias` - Remove aliases
- `source` - Execute script in current shell
- `array` - Array operations
- `plugin` - Plugin management
- `theme` - Theme management

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/winuxsh.git
cd winuxsh

# Build release version
cargo build --release

# The executable will be at target/release/mvp6-array.exe
```

### Setup

1. Add the executable directory to your PATH
2. Configure utils backend (default: winuxcmd)
3. Configure Windows Terminal to use WinSH as default shell

### Utils Backend Configuration

WinSH supports multiple Unix utils backends:

**Default Backend (WinuxCmd)**:
```bash
# Already configured in utils/winuxcmd/
# Available commands: ls, cat, grep, find, cp, mv, rm, mkdir, etc.
```

**Switching Backends**:
```bash
# Edit ~/.winshrc.toml
[utils]
backend = "winuxcmd"  # or "uutils"
path = "utils/winuxcmd"
```

For more details, see `utils/README.md`.

## Usage

add this to windows terminal settings
replace the default profile with this one and $env:PATH to your PATH variable
```json
{
    "guid": "{9acb9455-ca63-5af2-ba0c-1fa3a891bd59}",
    "commandline":"${env:PATH}\\winuxsh.exe",
    "hidden": false,
    "name": "winuxsh",
}
```

### Interactive Mode

```bash
./winuxsh.exe
```

### Execute Single Command

```bash
./winuxsh.exe -c "echo Hello World"
```

### Execute Script

```bash
./winuxsh.exe script.sh
```

### Command Examples

```bash
# Wildcard expansion
ls *.rs
echo *.toml

# Command substitution
echo "Current user: $(whoami)"

# Array operations
array define colors red green blue
array get colors 0
array len colors

# Theme management
theme list
theme set cyberpunk

# Plugin management
plugin list
```

## Architecture

WinSH follows a modular architecture with clear separation of concerns:

```
src/
├── main.rs           # Entry point and REPL loop
├── shell.rs          # Shell state and execution
├── tokenizer.rs      # Lexical analysis
├── parser.rs         # Syntax analysis
├── executor.rs       # Command execution
├── builtins.rs       # Built-in commands
├── array.rs          # Array system
├── plugin.rs         # Plugin system
├── theme.rs          # Theme management
├── config.rs         # Configuration
├── job.rs            # Job control
├── error.rs          # Error handling
└── oh_my_winuxsh.rs  # Oh-My-Winuxsh plugin
```

## Configuration

Configuration is stored in `~/.winshrc.toml`:

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {symbol}"

[theme]
current_theme = "default"

[aliases]
ll = "ls -la"
la = "ls -a"
```

## Theme System

WinSH includes 8 built-in themes:
- `default` - Classic green/blue theme
- `dark` - Minimal dark theme
- `light` - Light color theme
- `colorful` - Vibrant colors
- `minimal` - Plain text
- `cyberpunk` - Neon colors
- `ocean` - Blue tones
- `forest` - Green tones

## Plugin System

WinSH supports a plugin system for extending functionality:

### Built-in Plugins
- **Welcome Plugin**: Displays welcome message on startup
- **Oh-My-Winuxsh**: Theme and plugin management

### Creating Plugins

Implement the `Plugin` trait:

```rust
pub trait Plugin {
    fn name(&self) -> &str;
    fn init(&mut self) -> Result<()>;
    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool>;
    fn description(&self) -> &str;
}
```

## Compatibility

- **OS**: Windows 10/11
- **Rust**: 2021 edition
- **Terminal**: Windows Terminal recommended
- **Architecture**: x64

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- **reedline**: Line editing library by Nushell
- **winuxcmd**: Unix-style tools for Windows
- **colored**: Terminal color support

## Version History

### MVP6 (Current)
- Array support
- Plugin system
- Theme management
- 860+ command completion
- Full wildcard expansion
- Command substitution
- Script execution

### MVP5
- Job control
- Pipeline support
- Vi mode basics

### MVP4
- Basic shell functionality
- File operations
- Command execution

## Support

For issues and questions:
- GitHub Issues: https://github.com/yourusername/winuxsh/issues
- Documentation: See inline code documentation

## Roadmap

### MVP7 (Planned)
- Vi mode editing
- History search (Ctrl+R)
- Smart completion
- Pipeline improvements
- Background job control

### Future
- Cross-platform support (Linux, macOS)
- More plugins
- Advanced scripting features
- Performance optimizations