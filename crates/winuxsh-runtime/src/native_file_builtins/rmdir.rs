use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Eq, PartialEq)]
struct RmdirOptions {
    parents: bool,
    verbose: bool,
    ignore_fail_on_non_empty: bool,
}

pub(crate) fn execute_rmdir<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (options, operands) = match parse_rmdir_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        eprintln!("rmdir: missing operand");
        eprintln!("Try 'rmdir --help' for more information.");
        return 1;
    }

    let mut code = 0;
    for operand in operands {
        let path = resolve_path(&operand);
        if let Err(err) = remove_one(&operand, &path, &options) {
            if options.ignore_fail_on_non_empty && directory_not_empty(&err) {
                continue;
            }
            eprintln!(
                "rmdir: failed to remove '{}': {}",
                operand,
                rmdir_error(&err)
            );
            code = 1;
        }
    }
    code
}

fn remove_one(display: &str, path: &Path, options: &RmdirOptions) -> io::Result<()> {
    fs::remove_dir(path)?;
    if options.verbose {
        println!("rmdir: removing directory, '{}'", display);
    }
    if options.parents {
        remove_empty_parents(display, path, options);
    }
    Ok(())
}

fn remove_empty_parents(display: &str, path: &Path, options: &RmdirOptions) {
    let mut current = path.parent();
    let mut display_path = Path::new(display).parent();
    while let (Some(parent), Some(shown_path)) = (current, display_path) {
        if parent.as_os_str().is_empty() {
            break;
        }
        if fs::remove_dir(parent).is_err() {
            break;
        }
        if options.verbose {
            println!("rmdir: removing directory, '{}'", shown_path.display());
        }
        current = parent.parent();
        display_path = shown_path.parent();
    }
}

fn parse_rmdir_args(args: &[String]) -> Result<(RmdirOptions, Vec<String>), i32> {
    let mut options = RmdirOptions::default();
    let mut operands = Vec::new();
    let mut parse_options = true;

    for arg in args {
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }
        if parse_options && matches!(arg.as_str(), "-h" | "--help") {
            print_rmdir_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("rmdir (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg == "--ignore-fail-on-non-empty" {
            options.ignore_fail_on_non_empty = true;
            continue;
        }
        if parse_options && arg == "--parents" {
            options.parents = true;
            continue;
        }
        if parse_options && arg == "--verbose" {
            options.verbose = true;
            continue;
        }
        if parse_options && arg.starts_with("--") {
            eprintln!("rmdir: unrecognized option '{}'", arg);
            eprintln!("Try 'rmdir --help' for more information.");
            return Err(1);
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            for option in arg.chars().skip(1) {
                match option {
                    'p' => options.parents = true,
                    'v' => options.verbose = true,
                    other => {
                        eprintln!("rmdir: invalid option -- '{}'", other);
                        eprintln!("Try 'rmdir --help' for more information.");
                        return Err(1);
                    }
                }
            }
            continue;
        }
        operands.push(arg.clone());
    }

    Ok((options, operands))
}

fn directory_not_empty(err: &io::Error) -> bool {
    err.raw_os_error()
        .is_some_and(|code| matches!(code, 39 | 145))
        || err.to_string().to_ascii_lowercase().contains("not empty")
}

fn rmdir_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
        _ => err.to_string(),
    }
}

fn print_rmdir_help() {
    println!("Usage: rmdir [OPTION]... DIRECTORY...");
    println!("Remove the DIRECTORY(ies), if they are empty.");
    println!();
    println!("      --ignore-fail-on-non-empty  ignore non-empty directory failures");
    println!("  -p, --parents                   remove DIRECTORY and its ancestors");
    println!("  -v, --verbose                   output a diagnostic for every directory processed");
    println!("  -h, --help                      display this help and exit");
    println!("  -V, --version                   output version information and exit");
    println!("      --                          stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rmdir_common_options() {
        let (options, operands) = parse_rmdir_args(&[
            "-pv".into(),
            "--ignore-fail-on-non-empty".into(),
            "dir".into(),
        ])
        .unwrap();

        assert!(options.parents);
        assert!(options.verbose);
        assert!(options.ignore_fail_on_non_empty);
        assert_eq!(operands, vec!["dir"]);
    }
}
