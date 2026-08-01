use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Default, Eq, PartialEq)]
struct MkdirOptions {
    parents: bool,
    verbose: bool,
    mode: Option<String>,
    context: Option<String>,
}

pub(crate) fn execute_mkdir<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (options, operands) = match parse_mkdir_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        eprintln!("mkdir: missing operand");
        eprintln!("Try 'mkdir --help' for more information.");
        return 1;
    }

    let mut code = 0;
    for operand in operands {
        let path = resolve_path(&operand);
        let result = if options.parents {
            fs::create_dir_all(&path)
        } else {
            fs::create_dir(&path)
        };
        match result {
            Ok(()) => {
                if options.verbose {
                    println!("mkdir: created directory '{}'", operand);
                }
            }
            Err(err) => {
                if options.parents && path.is_dir() {
                    continue;
                }
                eprintln!(
                    "mkdir: cannot create directory '{}': {}",
                    operand,
                    mkdir_error(&err)
                );
                code = 1;
            }
        }
    }

    code
}

fn parse_mkdir_args(args: &[String]) -> Result<(MkdirOptions, Vec<String>), i32> {
    let mut options = MkdirOptions::default();
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
            print_mkdir_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("mkdir (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg.starts_with("--") {
            parse_mkdir_long_option(arg, args, &mut index, &mut options)?;
            continue;
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            parse_mkdir_short_options(arg, args, &mut index, &mut options)?;
            continue;
        }

        operands.push(arg.clone());
        index += 1;
    }

    Ok((options, operands))
}

fn parse_mkdir_long_option(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut MkdirOptions,
) -> Result<(), i32> {
    let (name, value) = split_long_option(arg);
    match name {
        "parents" => {
            reject_long_value("mkdir", arg, value)?;
            options.parents = true;
            *index += 1;
        }
        "verbose" => {
            reject_long_value("mkdir", arg, value)?;
            options.verbose = true;
            *index += 1;
        }
        "mode" => {
            options.mode = Some(take_long_value("mkdir", arg, value, args, index)?);
        }
        "context" => {
            options.context = Some(value.unwrap_or_default().to_string());
            *index += 1;
        }
        _ => {
            eprintln!("mkdir: unrecognized option '{}'", arg);
            eprintln!("Try 'mkdir --help' for more information.");
            return Err(1);
        }
    }
    Ok(())
}

fn parse_mkdir_short_options(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut MkdirOptions,
) -> Result<(), i32> {
    let chars: Vec<char> = arg.chars().collect();
    let mut pos = 1;
    while pos < chars.len() {
        match chars[pos] {
            'p' => options.parents = true,
            'v' => options.verbose = true,
            'Z' => options.context = Some(String::new()),
            'm' => {
                let value = if pos + 1 < chars.len() {
                    chars[pos + 1..].iter().collect()
                } else {
                    *index += 1;
                    let Some(value) = args.get(*index) else {
                        eprintln!("mkdir: option '-m' requires an argument");
                        eprintln!("Try 'mkdir --help' for more information.");
                        return Err(1);
                    };
                    value.clone()
                };
                options.mode = Some(value);
                *index += 1;
                return Ok(());
            }
            other => {
                eprintln!("mkdir: invalid option -- '{}'", other);
                eprintln!("Try 'mkdir --help' for more information.");
                return Err(1);
            }
        }
        pos += 1;
    }
    *index += 1;
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

fn mkdir_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::AlreadyExists => "File exists".to_string(),
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        _ => err.to_string(),
    }
}

fn print_mkdir_help() {
    println!("Usage: mkdir [OPTION]... DIRECTORY...");
    println!("make directories");
    println!();
    println!("  -m, --mode MODE     set file mode (accepted for compatibility)");
    println!("  -p, --parents       no error if existing, make parent directories as needed");
    println!("  -v, --verbose       print a message for each created directory");
    println!("  -Z, --context       accepted for SELinux compatibility");
    println!("  -h, --help          display this help and exit");
    println!("  -V, --version       output version information and exit");
    println!("      --              stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mkdir_common_options() {
        let (options, operands) = parse_mkdir_args(&[
            "-pv".into(),
            "-m".into(),
            "755".into(),
            "--".into(),
            "-dir".into(),
        ])
        .unwrap();

        assert!(options.parents);
        assert!(options.verbose);
        assert_eq!(options.mode.as_deref(), Some("755"));
        assert_eq!(operands, vec!["-dir"]);
    }
}
