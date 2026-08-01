use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Verbosity {
    Quiet,
    Changes,
    Verbose,
}

#[derive(Debug, Eq, PartialEq)]
struct ChmodOptions {
    recursive: bool,
    verbosity: Verbosity,
    silent: bool,
    preserve_root: bool,
    reference: Option<String>,
}

impl Default for ChmodOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            verbosity: Verbosity::Quiet,
            silent: false,
            preserve_root: true,
            reference: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModeChange {
    SetReadonly(bool),
    Noop,
}

pub(crate) fn execute_chmod<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (options, operands) = match parse_chmod_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    let (change, files) = match build_mode_change(&options, &operands, &mut resolve_path) {
        Ok(value) => value,
        Err(code) => return code,
    };

    let mut code = 0;
    for file in files {
        let path = resolve_path(&file);
        if options.recursive && options.preserve_root && is_root_path(&path) {
            if !options.silent {
                eprintln!(
                    "chmod: it is dangerous to operate recursively on '{}'",
                    file
                );
            }
            code = 1;
            continue;
        }
        if let Err(err) = chmod_path(&file, &path, change, &options) {
            if !options.silent {
                eprintln!("chmod: cannot access '{}': {}", file, chmod_error(&err));
            }
            code = 1;
        }
    }
    code
}

fn build_mode_change<F>(
    options: &ChmodOptions,
    operands: &[String],
    resolve_path: &mut F,
) -> Result<(ModeChange, Vec<String>), i32>
where
    F: FnMut(&str) -> PathBuf,
{
    if let Some(reference) = &options.reference {
        if operands.is_empty() {
            eprintln!("chmod: missing operand after '--reference'");
            eprintln!("Try 'chmod --help' for more information.");
            return Err(1);
        }
        let path = resolve_path(reference);
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!(
                    "chmod: failed to get attributes of '{}': {}",
                    reference,
                    chmod_error(&err)
                );
                return Err(1);
            }
        };
        return Ok((
            ModeChange::SetReadonly(metadata.permissions().readonly()),
            operands.to_vec(),
        ));
    }

    let Some((mode, files)) = operands.split_first() else {
        eprintln!("chmod: missing operand");
        eprintln!("Try 'chmod --help' for more information.");
        return Err(1);
    };
    if files.is_empty() {
        eprintln!("chmod: missing operand after '{}'", mode);
        eprintln!("Try 'chmod --help' for more information.");
        return Err(1);
    }
    Ok((parse_mode_change(mode)?, files.to_vec()))
}

fn parse_chmod_args(args: &[String]) -> Result<(ChmodOptions, Vec<String>), i32> {
    let mut options = ChmodOptions::default();
    let mut operands = Vec::new();
    let mut parse_options = true;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        if parse_options && arg == "--" {
            parse_options = false;
            index += 1;
            continue;
        }
        if parse_options && matches!(arg.as_str(), "-h" | "--help") {
            print_chmod_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("chmod (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg.starts_with("--") {
            parse_chmod_long_option(arg, args, &mut index, &mut options)?;
            continue;
        }
        if parse_options && arg.starts_with('-') && arg != "-" && known_chmod_short_options(arg) {
            parse_chmod_short_options(arg, &mut options)?;
            index += 1;
            continue;
        }

        operands.extend(args[index..].iter().cloned());
        break;
    }

    Ok((options, operands))
}

fn known_chmod_short_options(arg: &str) -> bool {
    arg.chars()
        .skip(1)
        .all(|ch| matches!(ch, 'c' | 'f' | 'v' | 'R' | 'H' | 'L' | 'P'))
}

fn parse_chmod_long_option(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut ChmodOptions,
) -> Result<(), i32> {
    let (name, value) = split_long_option(arg);
    match name {
        "changes" => {
            reject_long_value("chmod", arg, value)?;
            options.verbosity = Verbosity::Changes;
            *index += 1;
        }
        "silent" | "quiet" => {
            reject_long_value("chmod", arg, value)?;
            options.silent = true;
            *index += 1;
        }
        "verbose" => {
            reject_long_value("chmod", arg, value)?;
            options.verbosity = Verbosity::Verbose;
            *index += 1;
        }
        "recursive" => {
            reject_long_value("chmod", arg, value)?;
            options.recursive = true;
            *index += 1;
        }
        "reference" => {
            options.reference = Some(take_long_value("chmod", arg, value, args, index)?);
        }
        "dereference" | "preserve-root" => {
            reject_long_value("chmod", arg, value)?;
            options.preserve_root = true;
            *index += 1;
        }
        "no-preserve-root" => {
            reject_long_value("chmod", arg, value)?;
            options.preserve_root = false;
            *index += 1;
        }
        _ => {
            eprintln!("chmod: unrecognized option '{}'", arg);
            eprintln!("Try 'chmod --help' for more information.");
            return Err(1);
        }
    }
    Ok(())
}

fn parse_chmod_short_options(arg: &str, options: &mut ChmodOptions) -> Result<(), i32> {
    for option in arg.chars().skip(1) {
        match option {
            'c' => options.verbosity = Verbosity::Changes,
            'f' => options.silent = true,
            'v' => options.verbosity = Verbosity::Verbose,
            'R' => options.recursive = true,
            'H' | 'L' | 'P' => {}
            other => {
                eprintln!("chmod: invalid option -- '{}'", other);
                eprintln!("Try 'chmod --help' for more information.");
                return Err(1);
            }
        }
    }
    Ok(())
}

fn parse_mode_change(mode: &str) -> Result<ModeChange, i32> {
    if mode.is_empty() {
        eprintln!("chmod: invalid mode: '{}'", mode);
        return Err(1);
    }
    if mode.chars().all(|ch| matches!(ch, '0'..='7')) {
        let value = u32::from_str_radix(mode, 8).map_err(|_| {
            eprintln!("chmod: invalid mode: '{}'", mode);
            1
        })?;
        return Ok(ModeChange::SetReadonly(value & 0o222 == 0));
    }

    let mut change = None;
    for clause in mode.split(',') {
        if clause.is_empty() {
            eprintln!("chmod: invalid mode: '{}'", mode);
            return Err(1);
        }
        let mut chars = clause.char_indices().peekable();
        while let Some((offset, ch)) = chars.next() {
            if !matches!(ch, '+' | '-' | '=') {
                continue;
            }
            let permissions_start = offset + ch.len_utf8();
            let end = clause[permissions_start..]
                .char_indices()
                .find(|(_, next_ch)| matches!(next_ch, '+' | '-' | '='))
                .map(|(next_offset, _)| permissions_start + next_offset)
                .unwrap_or(clause.len());
            let permissions = &clause[permissions_start..end];
            match ch {
                '+' if permissions.contains('w') => change = Some(false),
                '-' if permissions.contains('w') => change = Some(true),
                '=' => change = Some(!permissions.contains('w')),
                _ => {}
            }
        }
    }

    Ok(change
        .map(ModeChange::SetReadonly)
        .unwrap_or(ModeChange::Noop))
}

fn chmod_path(
    display: &str,
    path: &Path,
    change: ModeChange,
    options: &ChmodOptions,
) -> io::Result<()> {
    apply_chmod_one(display, path, change, options)?;
    if options.recursive && path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            let child_display = format!(
                "{}/{}",
                display.trim_end_matches(['/', '\\']),
                entry.file_name().to_string_lossy()
            );
            chmod_path(&child_display, &child_path, change, options)?;
        }
    }
    Ok(())
}

fn apply_chmod_one(
    display: &str,
    path: &Path,
    change: ModeChange,
    options: &ChmodOptions,
) -> io::Result<()> {
    let ModeChange::SetReadonly(readonly) = change else {
        return Ok(());
    };
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    let before = permissions.readonly();
    permissions.set_readonly(readonly);
    if before != readonly {
        fs::set_permissions(path, permissions)?;
    }
    if options.verbosity == Verbosity::Verbose
        || (options.verbosity == Verbosity::Changes && before != readonly)
    {
        println!("mode of '{}' changed", display);
    }
    Ok(())
}

fn split_long_option(arg: &str) -> (&str, Option<&str>) {
    arg.strip_prefix("--")
        .unwrap_or(arg)
        .split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or_else(|| (arg.strip_prefix("--").unwrap_or(arg), None))
}

fn reject_long_value(command: &str, arg: &str, value: Option<&str>) -> Result<(), i32> {
    if value.is_some() {
        eprintln!("{}: option '{}' doesn't allow an argument", command, arg);
        eprintln!("Try '{} --help' for more information.", command);
        return Err(1);
    }
    Ok(())
}

fn take_long_value(
    command: &str,
    arg: &str,
    value: Option<&str>,
    args: &[String],
    index: &mut usize,
) -> Result<String, i32> {
    if let Some(value) = value {
        *index += 1;
        return Ok(value.to_string());
    }
    *index += 1;
    let Some(value) = args.get(*index) else {
        eprintln!("{}: option '{}' requires an argument", command, arg);
        eprintln!("Try '{} --help' for more information.", command);
        return Err(1);
    };
    *index += 1;
    Ok(value.clone())
}

fn is_root_path(path: &Path) -> bool {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(_)) => {
            matches!(components.next(), Some(Component::RootDir)) && components.next().is_none()
        }
        Some(Component::RootDir) => components.next().is_none(),
        _ => false,
    }
}

fn chmod_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        _ => err.to_string(),
    }
}

fn print_chmod_help() {
    println!("Usage: chmod [OPTION]... MODE FILE...");
    println!("Change the mode of each FILE to MODE.");
    println!();
    println!("On Windows, write permission maps to the read-only file attribute.");
    println!();
    println!("  -c, --changes           report only when a change is made");
    println!("  -f, --silent, --quiet   suppress most error messages");
    println!("  -v, --verbose           output a diagnostic for every file processed");
    println!("  -R, --recursive         change files and directories recursively");
    println!("      --reference RFILE   use RFILE's mode instead of MODE values");
    println!("      --preserve-root     fail to operate recursively on root");
    println!("      --no-preserve-root  do not treat root specially");
    println!("  -H, -L, -P              accepted for compatibility");
    println!("  -h, --help              display this help and exit");
    println!("  -V, --version           output version information and exit");
    println!("      --                  stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chmod_mode_that_starts_with_dash() {
        let (options, operands) = parse_chmod_args(&["-w".into(), "file".into()]).unwrap();
        assert_eq!(options, ChmodOptions::default());
        assert_eq!(operands, vec!["-w", "file"]);
        assert_eq!(
            parse_mode_change("-w").unwrap(),
            ModeChange::SetReadonly(true)
        );
    }

    #[test]
    fn parses_chmod_recursive_and_numeric_mode() {
        let (options, operands) =
            parse_chmod_args(&["-Rv".into(), "755".into(), "dir".into()]).unwrap();
        assert!(options.recursive);
        assert_eq!(options.verbosity, Verbosity::Verbose);
        assert_eq!(operands, vec!["755", "dir"]);
        assert_eq!(
            parse_mode_change("755").unwrap(),
            ModeChange::SetReadonly(false)
        );
    }
}
