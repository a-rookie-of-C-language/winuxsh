// Semantics are adapted from uutils coreutils `rm` while keeping winuxsh's
// native builtins lightweight and independent of uucore's CLI framework.

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InteractiveMode {
    Never,
    Once,
    Always,
    PromptProtected,
}

#[derive(Debug, PartialEq)]
struct RmOptions {
    force: bool,
    interactive: InteractiveMode,
    one_file_system: bool,
    preserve_root: bool,
    preserve_root_all: bool,
    recursive: bool,
    dir: bool,
    verbose: bool,
    progress: bool,
}

impl Default for RmOptions {
    fn default() -> Self {
        Self {
            force: false,
            interactive: InteractiveMode::PromptProtected,
            one_file_system: false,
            preserve_root: true,
            preserve_root_all: false,
            recursive: false,
            dir: false,
            verbose: false,
            progress: false,
        }
    }
}

pub(crate) fn execute_rm<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (options, operands) = match parse_rm_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        if options.force {
            return 0;
        }
        eprintln!("rm: missing operand");
        eprintln!("Try 'rm --help' for more information.");
        return 1;
    }

    if options.interactive == InteractiveMode::Once && (options.recursive || operands.len() > 3) {
        let noun = if operands.len() == 1 {
            "argument"
        } else {
            "arguments"
        };
        let suffix = if options.recursive {
            " recursively"
        } else {
            ""
        };
        if !prompt_yes(&format!(
            "rm: remove {} {}{}? ",
            operands.len(),
            noun,
            suffix
        )) {
            return 0;
        }
    }

    let mut exit_code = 0;
    for operand in operands {
        let target = resolve_path(&operand);
        if let Err(err) = remove_operand(&operand, &target, &options) {
            if options.force && err.kind() == io::ErrorKind::NotFound {
                continue;
            }
            eprintln!(
                "rm: cannot remove '{}': {}",
                operand,
                rm_error_message(&err)
            );
            exit_code = 1;
        }
    }

    exit_code
}

fn parse_rm_args(args: &[String]) -> Result<(RmOptions, Vec<String>), i32> {
    let mut options = RmOptions::default();
    let mut operands = Vec::new();
    let mut parse_options = true;

    for arg in args {
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }

        if parse_options && arg.starts_with("--") {
            parse_rm_long_option(arg, &mut options)?;
            continue;
        }

        if parse_options && arg.starts_with('-') && arg != "-" {
            parse_rm_short_options(arg, &mut options)?;
            continue;
        }

        operands.push(arg.clone());
    }

    Ok((options, operands))
}

fn parse_rm_long_option(arg: &str, options: &mut RmOptions) -> Result<(), i32> {
    if arg == "--help" {
        print_rm_help();
        return Err(0);
    }
    if arg == "--version" {
        println!("rm (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
        return Err(0);
    }
    let raw_name = arg
        .strip_prefix("--")
        .unwrap_or(arg)
        .split_once('=')
        .map(|(name, _)| name)
        .unwrap_or_else(|| arg.strip_prefix("--").unwrap_or(arg));
    if "no-preserve-root".starts_with(raw_name) && raw_name != "no-preserve-root" {
        eprintln!("rm: option '--no-preserve-root' may not be abbreviated");
        return Err(2);
    }

    let (name, value) = arg
        .strip_prefix("--")
        .unwrap_or(arg)
        .split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or_else(|| (arg.strip_prefix("--").unwrap_or(arg), None));
    let Some(canonical) = unique_long_option(name) else {
        eprintln!("rm: unrecognized option '{}'", arg);
        eprintln!("Try 'rm --help' for more information.");
        return Err(2);
    };

    match canonical {
        "force" => {
            reject_unexpected_value(arg, value)?;
            options.force = true;
            options.interactive = InteractiveMode::Never;
        }
        "interactive" => {
            options.interactive = parse_interactive_value(value.unwrap_or("always"))?;
        }
        "one-file-system" => {
            reject_unexpected_value(arg, value)?;
            options.one_file_system = true;
        }
        "preserve-root" => {
            options.preserve_root = true;
            options.preserve_root_all = match value {
                None => false,
                Some("all") => true,
                Some(other) => {
                    eprintln!("rm: invalid argument '{}' for '--preserve-root'", other);
                    return Err(2);
                }
            };
        }
        "no-preserve-root" => {
            reject_unexpected_value(arg, value)?;
            options.preserve_root = false;
            options.preserve_root_all = false;
        }
        "recursive" => {
            reject_unexpected_value(arg, value)?;
            options.recursive = true;
        }
        "dir" => {
            reject_unexpected_value(arg, value)?;
            options.dir = true;
        }
        "verbose" => {
            reject_unexpected_value(arg, value)?;
            options.verbose = true;
        }
        "progress" => {
            reject_unexpected_value(arg, value)?;
            options.progress = true;
        }
        _ => unreachable!("unique_long_option returned an unknown rm option"),
    }

    Ok(())
}

fn parse_rm_short_options(arg: &str, options: &mut RmOptions) -> Result<(), i32> {
    for option in arg.chars().skip(1) {
        match option {
            'f' => {
                options.force = true;
                options.interactive = InteractiveMode::Never;
            }
            'i' => options.interactive = InteractiveMode::Always,
            'I' => options.interactive = InteractiveMode::Once,
            'r' | 'R' => options.recursive = true,
            'd' => options.dir = true,
            'v' => options.verbose = true,
            'g' => options.progress = true,
            other => {
                eprintln!("rm: invalid option -- '{}'", other);
                if Path::new(arg).exists() {
                    eprintln!("Try 'rm ./{}' to remove the file '{}'.", arg, arg);
                }
                eprintln!("Try 'rm --help' for more information.");
                return Err(2);
            }
        }
    }

    Ok(())
}

fn unique_long_option(name: &str) -> Option<&'static str> {
    const OPTIONS: &[&str] = &[
        "force",
        "interactive",
        "one-file-system",
        "preserve-root",
        "no-preserve-root",
        "recursive",
        "dir",
        "verbose",
        "progress",
    ];
    let mut matches = OPTIONS
        .iter()
        .copied()
        .filter(|option| option.starts_with(name));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn reject_unexpected_value(arg: &str, value: Option<&str>) -> Result<(), i32> {
    if value.is_some() {
        eprintln!("rm: option '{}' doesn't allow an argument", arg);
        return Err(2);
    }
    Ok(())
}

fn parse_interactive_value(value: &str) -> Result<InteractiveMode, i32> {
    let canonical = match value {
        "always" | "yes" => "always",
        "once" => "once",
        "never" | "no" | "none" => "never",
        prefix => {
            let mut matches = ["always", "once", "never"]
                .into_iter()
                .filter(|candidate| candidate.starts_with(prefix));
            let Some(first) = matches.next() else {
                eprintln!("rm: invalid argument '{}' for '--interactive'", value);
                return Err(2);
            };
            if matches.next().is_some() {
                eprintln!("rm: ambiguous argument '{}' for '--interactive'", value);
                return Err(2);
            }
            first
        }
    };

    Ok(match canonical {
        "always" => InteractiveMode::Always,
        "once" => InteractiveMode::Once,
        "never" => InteractiveMode::Never,
        _ => unreachable!("interactive value canonicalization failed"),
    })
}

fn remove_operand(display: &str, target: &Path, options: &RmOptions) -> io::Result<()> {
    let cleaned_display = clean_trailing_separators(display);
    if path_is_current_or_parent_directory(&cleaned_display) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "refusing to remove '.' or '..' directory",
        ));
    }

    let metadata = fs::symlink_metadata(target)?;
    let file_type = metadata.file_type();

    if file_type.is_dir() && !file_type.is_symlink() {
        return remove_directory(display, target, options);
    }

    if prompt_file(display, target, &metadata, options) {
        fs::remove_file(target)?;
        verbose_removed(display, false, options);
    }

    Ok(())
}

fn remove_directory(display: &str, target: &Path, options: &RmOptions) -> io::Result<()> {
    let root = is_root_path(target);
    if root && options.preserve_root {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "it is dangerous to operate recursively on root",
        ));
    }

    if options.recursive {
        if prompt_directory(display, options) {
            // TODO: match uutils/GNU --one-file-system semantics. We accept the
            // flag for compatibility, but Windows/native traversal is still
            // the stdlib recursive removal path for now.
            fs::remove_dir_all(target)?;
            verbose_removed(display, true, options);
        }
        return Ok(());
    }

    if options.dir {
        if prompt_directory(display, options) {
            fs::remove_dir(target)?;
            verbose_removed(display, true, options);
        }
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::IsADirectory,
        "Is a directory",
    ))
}

fn prompt_file(display: &str, target: &Path, metadata: &fs::Metadata, options: &RmOptions) -> bool {
    match options.interactive {
        InteractiveMode::Never => true,
        InteractiveMode::Always => prompt_yes(&format!("rm: remove file '{}'? ", display)),
        InteractiveMode::Once => true,
        InteractiveMode::PromptProtected => {
            if stdin_is_terminal() && is_write_protected(metadata) {
                prompt_yes(&format!("rm: remove write-protected file '{}'? ", display))
            } else if target.as_os_str().is_empty() {
                false
            } else {
                true
            }
        }
    }
}

fn prompt_directory(display: &str, options: &RmOptions) -> bool {
    match options.interactive {
        InteractiveMode::Never | InteractiveMode::PromptProtected | InteractiveMode::Once => true,
        InteractiveMode::Always => prompt_yes(&format!("rm: remove directory '{}'? ", display)),
    }
}

fn stdin_is_terminal() -> bool {
    std::io::stdin().is_terminal()
}

fn prompt_yes(prompt: &str) -> bool {
    if !stdin_is_terminal() {
        return false;
    }
    eprint!("{}", prompt);
    let _ = io::stderr().flush();
    let mut answer = String::new();
    match io::stdin().read_line(&mut answer) {
        Ok(_) => matches!(answer.trim(), "y" | "Y" | "yes" | "YES" | "Yes"),
        Err(_) => false,
    }
}

fn is_write_protected(metadata: &fs::Metadata) -> bool {
    metadata.permissions().readonly()
}

fn is_root_path(path: &Path) -> bool {
    is_filesystem_root(path)
        || path
            .canonicalize()
            .is_ok_and(|canonical| is_filesystem_root(&canonical))
}

fn is_filesystem_root(path: &Path) -> bool {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(_)) => {
            matches!(components.next(), Some(Component::RootDir)) && components.next().is_none()
        }
        Some(Component::RootDir) => components.next().is_none(),
        _ => false,
    }
}

fn path_is_current_or_parent_directory(path: &str) -> bool {
    let trimmed = clean_trailing_separators(path);
    let basename = trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(trimmed.as_str());
    basename == "." || basename == ".."
}

fn clean_trailing_separators(path: &str) -> String {
    if path.len() <= 1 {
        return path.to_string();
    }
    let mut end = path.len();
    let bytes = path.as_bytes();
    while end > 1 {
        let ch = bytes[end - 1] as char;
        if ch != '/' && ch != '\\' && ch != MAIN_SEPARATOR {
            break;
        }
        end -= 1;
    }
    path[..end].to_string()
}

fn verbose_removed(display: &str, directory: bool, options: &RmOptions) {
    if !options.verbose {
        return;
    }
    if directory {
        println!("removed directory '{}'", display);
    } else {
        println!("removed '{}'", display);
    }
}

fn rm_error_message(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        io::ErrorKind::IsADirectory => "Is a directory".to_string(),
        _ => err.to_string(),
    }
}

fn print_rm_help() {
    println!("Usage: rm [OPTION]... [FILE]...");
    println!("Remove files or directories.");
    println!();
    println!("  -f, --force              ignore nonexistent files and arguments");
    println!("  -i                       prompt before every removal");
    println!("  -I                       prompt once before recursive or bulk removal");
    println!("      --interactive[=WHEN] prompt according to WHEN: never, once, always");
    println!("      --one-file-system    accepted for GNU compatibility");
    println!("      --no-preserve-root   do not treat root specially");
    println!("      --preserve-root[=all] do not remove root recursively");
    println!("  -r, -R, --recursive      remove directories and their contents");
    println!("  -d, --dir                remove empty directories");
    println!("  -v, --verbose            explain what is being done");
    println!("  -g, --progress           accepted for uutils compatibility");
    println!("      --                   stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_short_options_follow_last_occurrence() {
        let (options, _) = parse_rm_args(&["-i".into(), "-f".into()]).unwrap();
        assert_eq!(options.interactive, InteractiveMode::Never);

        let (options, _) = parse_rm_args(&["-f".into(), "-i".into()]).unwrap();
        assert_eq!(options.interactive, InteractiveMode::Always);
    }

    #[test]
    fn parse_interactive_long_values_and_prefixes() {
        let (options, _) = parse_rm_args(&["--interactive=once".into()]).unwrap();
        assert_eq!(options.interactive, InteractiveMode::Once);

        let (options, _) = parse_rm_args(&["--interactive=n".into()]).unwrap();
        assert_eq!(options.interactive, InteractiveMode::Never);

        let (options, _) = parse_rm_args(&["--interactive".into()]).unwrap();
        assert_eq!(options.interactive, InteractiveMode::Always);
    }

    #[test]
    fn no_preserve_root_may_not_be_abbreviated() {
        assert_eq!(parse_rm_args(&["--no-preserve-roo".into()]), Err(2));
    }

    #[test]
    fn rm_removes_dash_prefixed_operand_after_separator() {
        let temp = unique_temp_dir("winuxsh-native-rm-dash");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("-p"), "payload").unwrap();

        let code = execute_rm(&["-rf".into(), "--".into(), "-p".into()], |arg| {
            temp.join(arg)
        });

        assert_eq!(code, 0);
        assert!(!temp.join("-p").exists());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rm_refuses_directory_without_recursive_or_dir() {
        let temp = unique_temp_dir("winuxsh-native-rm-dir");
        let dir = temp.join("dir");
        fs::create_dir_all(&dir).unwrap();

        let code = execute_rm(&["dir".into()], |arg| temp.join(arg));

        assert_eq!(code, 1);
        assert!(dir.exists());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rm_recursive_removes_directory_tree() {
        let temp = unique_temp_dir("winuxsh-native-rm-recursive");
        let dir = temp.join("dir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), "payload").unwrap();

        let code = execute_rm(&["-r".into(), "dir".into()], |arg| temp.join(arg));

        assert_eq!(code, 0);
        assert!(!dir.exists());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rm_refuses_current_and_parent_directory_operands() {
        let temp = unique_temp_dir("winuxsh-native-rm-current-parent");
        fs::create_dir_all(&temp).unwrap();

        let current = execute_rm(&[".".into()], |_| temp.clone());
        let parent = execute_rm(&["foo/..//".into()], |_| temp.clone());

        assert_eq!(current, 1);
        assert_eq!(parent, 1);
        assert!(temp.exists());
        let _ = fs::remove_dir_all(temp);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
    }
}
