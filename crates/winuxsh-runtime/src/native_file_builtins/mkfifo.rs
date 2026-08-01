use std::path::PathBuf;

#[derive(Debug, Default, Eq, PartialEq)]
struct MkfifoOptions {
    mode: Option<String>,
    context: Option<String>,
}

pub(crate) fn execute_mkfifo<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (_options, operands) = match parse_mkfifo_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        eprintln!("mkfifo: missing operand");
        eprintln!("Try 'mkfifo --help' for more information.");
        return 1;
    }

    let mut code = 0;
    for operand in operands {
        let _ = resolve_path(&operand);
        eprintln!(
            "mkfifo: cannot create fifo '{}': filesystem FIFOs are not supported on Windows",
            operand
        );
        code = 1;
    }
    code
}

fn parse_mkfifo_args(args: &[String]) -> Result<(MkfifoOptions, Vec<String>), i32> {
    let mut options = MkfifoOptions::default();
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
            print_mkfifo_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("mkfifo (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg.starts_with("--mode") {
            options.mode = Some(take_option_value(
                "mkfifo", arg, "--mode", args, &mut index,
            )?);
            continue;
        }
        if parse_options && arg == "--context" {
            options.context = Some(String::new());
            index += 1;
            continue;
        }
        if parse_options && arg.starts_with("--context=") {
            options.context = Some(arg.trim_start_matches("--context=").to_string());
            index += 1;
            continue;
        }
        if parse_options && arg.starts_with("--") {
            eprintln!("mkfifo: unrecognized option '{}'", arg);
            eprintln!("Try 'mkfifo --help' for more information.");
            return Err(1);
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            let chars: Vec<char> = arg.chars().collect();
            let mut pos = 1;
            while pos < chars.len() {
                match chars[pos] {
                    'Z' => options.context = Some(String::new()),
                    'm' => {
                        let value = if pos + 1 < chars.len() {
                            chars[pos + 1..].iter().collect()
                        } else {
                            index += 1;
                            let Some(value) = args.get(index) else {
                                eprintln!("mkfifo: option '-m' requires an argument");
                                eprintln!("Try 'mkfifo --help' for more information.");
                                return Err(1);
                            };
                            value.clone()
                        };
                        options.mode = Some(value);
                        index += 1;
                        break;
                    }
                    other => {
                        eprintln!("mkfifo: invalid option -- '{}'", other);
                        eprintln!("Try 'mkfifo --help' for more information.");
                        return Err(1);
                    }
                }
                pos += 1;
            }
            if pos >= chars.len() {
                index += 1;
            }
            continue;
        }

        operands.push(arg.clone());
        index += 1;
    }

    Ok((options, operands))
}

fn take_option_value(
    command: &str,
    arg: &str,
    name: &str,
    args: &[String],
    index: &mut usize,
) -> Result<String, i32> {
    if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
        *index += 1;
        return Ok(value.to_string());
    }
    if arg != name {
        eprintln!("{}: unrecognized option '{}'", command, arg);
        eprintln!("Try '{} --help' for more information.", command);
        return Err(1);
    }
    *index += 1;
    let Some(value) = args.get(*index) else {
        eprintln!("{}: option '{}' requires an argument", command, name);
        eprintln!("Try '{} --help' for more information.", command);
        return Err(1);
    };
    *index += 1;
    Ok(value.clone())
}

fn print_mkfifo_help() {
    println!("Usage: mkfifo [OPTION]... NAME...");
    println!("Create named pipes (FIFOs) with the given NAMEs.");
    println!();
    println!("Winuxsh accepts the GNU-compatible command line surface, but Windows");
    println!("does not provide filesystem FIFOs equivalent to POSIX named pipes.");
    println!();
    println!("  -m, --mode MODE  set file permission bits");
    println!("  -Z               accepted for SELinux compatibility");
    println!("      --context    accepted for SELinux compatibility");
    println!("  -h, --help       display this help and exit");
    println!("  -V, --version    output version information and exit");
    println!("      --           stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mkfifo_options_and_dash_operand() {
        let (options, operands) =
            parse_mkfifo_args(&["-m".into(), "600".into(), "--".into(), "-pipe".into()]).unwrap();

        assert_eq!(options.mode.as_deref(), Some("600"));
        assert_eq!(operands, vec!["-pipe"]);
    }
}
