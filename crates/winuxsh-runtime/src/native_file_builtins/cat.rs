// Semantics are adapted from uutils/winuxcmd `cat`, but this stays a small
// winuxsh-native implementation with no uucore/clap dependency.

use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NumberMode {
    None,
    NonBlank,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CatOptions {
    number: NumberMode,
    squeeze_blank: bool,
    show_ends: bool,
    show_tabs: bool,
    show_nonprinting: bool,
}

impl Default for CatOptions {
    fn default() -> Self {
        Self {
            number: NumberMode::None,
            squeeze_blank: false,
            show_ends: false,
            show_tabs: false,
            show_nonprinting: false,
        }
    }
}

impl CatOptions {
    fn can_copy_fast(self) -> bool {
        self.number == NumberMode::None
            && !self.squeeze_blank
            && !self.show_ends
            && !self.show_tabs
            && !self.show_nonprinting
    }
}

#[derive(Debug)]
struct CatState {
    line_number: usize,
    at_line_start: bool,
    kept_blank_line: bool,
}

impl Default for CatState {
    fn default() -> Self {
        Self {
            line_number: 1,
            at_line_start: true,
            kept_blank_line: false,
        }
    }
}

pub(crate) fn execute_cat<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let stdin = io::stdin();
    let stdout = io::stdout();
    execute_cat_with_io(
        args,
        &mut resolve_path,
        &mut stdin.lock(),
        &mut stdout.lock(),
    )
}

fn execute_cat_with_io<F, R, W>(
    args: &[String],
    resolve_path: &mut F,
    stdin: &mut R,
    stdout: &mut W,
) -> i32
where
    F: FnMut(&str) -> PathBuf,
    R: Read,
    W: Write,
{
    let (options, mut operands) = match parse_cat_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        operands.push("-".to_string());
    }

    let mut code = 0;
    let mut state = CatState::default();
    for operand in operands {
        let result = if operand == "-" {
            write_reader(stdin, stdout, options, &mut state)
        } else {
            let path = resolve_path(&operand);
            write_file(&operand, path, stdout, options, &mut state)
        };

        if let Err(err) = result {
            if err.kind() == io::ErrorKind::BrokenPipe {
                return code;
            }
            eprintln!("cat: {}: {}", operand, cat_error_message(&err));
            code = 1;
        }
    }

    code
}

fn parse_cat_args(args: &[String]) -> Result<(CatOptions, Vec<String>), i32> {
    let mut options = CatOptions::default();
    let mut operands = Vec::new();
    let mut parse_options = true;

    for arg in args {
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }

        if parse_options && arg.starts_with("--") {
            parse_cat_long_option(arg, &mut options)?;
            continue;
        }

        if parse_options && arg.starts_with('-') && arg != "-" {
            parse_cat_short_options(arg, &mut options)?;
            continue;
        }

        operands.push(arg.clone());
    }

    Ok((options, operands))
}

fn parse_cat_long_option(arg: &str, options: &mut CatOptions) -> Result<(), i32> {
    if arg == "--help" {
        print_cat_help();
        return Err(0);
    }
    if arg == "--version" {
        println!("cat (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
        return Err(0);
    }

    let (name, value) = arg
        .strip_prefix("--")
        .unwrap_or(arg)
        .split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or_else(|| (arg.strip_prefix("--").unwrap_or(arg), None));

    if value.is_some() {
        eprintln!("cat: option '{}' doesn't allow an argument", arg);
        eprintln!("Try 'cat --help' for more information.");
        return Err(1);
    }

    let short = match name {
        "show-all" => 'A',
        "number-nonblank" => 'b',
        "show-ends" => 'E',
        "number" => 'n',
        "squeeze-blank" => 's',
        "show-tabs" => 'T',
        "show-nonprinting" => 'v',
        _ => {
            eprintln!("cat: unrecognized option '{}'", arg);
            eprintln!("Try 'cat --help' for more information.");
            return Err(1);
        }
    };
    let _ = apply_cat_short_option(short, options);
    Ok(())
}

fn parse_cat_short_options(arg: &str, options: &mut CatOptions) -> Result<(), i32> {
    for option in arg.chars().skip(1) {
        if option == 'h' {
            print_cat_help();
            return Err(0);
        }
        if option == 'V' {
            println!("cat (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }

        match apply_cat_short_option(option, options) {
            Ok(()) => {}
            Err(()) => {
                eprintln!("cat: unrecognized option '-{}'", option);
                eprintln!("Try 'cat --help' for more information.");
                return Err(1);
            }
        }
    }
    Ok(())
}

fn apply_cat_short_option(option: char, options: &mut CatOptions) -> Result<(), ()> {
    match option {
        'A' => {
            options.show_nonprinting = true;
            options.show_ends = true;
            options.show_tabs = true;
        }
        'b' => options.number = NumberMode::NonBlank,
        'e' => {
            options.show_nonprinting = true;
            options.show_ends = true;
        }
        'E' => options.show_ends = true,
        'n' => {
            if options.number != NumberMode::NonBlank {
                options.number = NumberMode::All;
            }
        }
        's' => options.squeeze_blank = true,
        't' => {
            options.show_nonprinting = true;
            options.show_tabs = true;
        }
        'T' => options.show_tabs = true,
        'u' => {}
        'v' => options.show_nonprinting = true,
        _ => return Err(()),
    }
    Ok(())
}

fn write_file<W: Write>(
    _display: &str,
    path: PathBuf,
    stdout: &mut W,
    options: CatOptions,
    state: &mut CatState,
) -> io::Result<()> {
    if fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Is a directory",
        ));
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    write_reader(&mut reader, stdout, options, state)
}

fn write_reader<R: Read, W: Write>(
    reader: &mut R,
    stdout: &mut W,
    options: CatOptions,
    state: &mut CatState,
) -> io::Result<()> {
    if options.can_copy_fast() {
        io::copy(reader, stdout)?;
        return Ok(());
    }

    let mut buffer = [0_u8; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(());
        }

        for &byte in &buffer[..bytes_read] {
            write_transformed_byte(byte, stdout, options, state)?;
        }
    }
}

fn write_transformed_byte<W: Write>(
    byte: u8,
    stdout: &mut W,
    options: CatOptions,
    state: &mut CatState,
) -> io::Result<()> {
    let blank_line = state.at_line_start && byte == b'\n';
    if blank_line && options.squeeze_blank && state.kept_blank_line {
        return Ok(());
    }

    if byte != b'\n' {
        state.kept_blank_line = false;
    }

    if state.at_line_start {
        let should_number = options.number == NumberMode::All
            || (options.number == NumberMode::NonBlank && !blank_line);
        if should_number {
            write!(stdout, "{:>6}\t", state.line_number)?;
            state.line_number += 1;
        }
    }

    if byte == b'\n' {
        if blank_line {
            state.kept_blank_line = true;
        }
        if options.show_ends {
            stdout.write_all(b"$")?;
        }
        stdout.write_all(b"\n")?;
        state.at_line_start = true;
        return Ok(());
    }

    state.at_line_start = false;
    write_visible_byte(byte, stdout, options)
}

fn write_visible_byte<W: Write>(byte: u8, stdout: &mut W, options: CatOptions) -> io::Result<()> {
    if byte == b'\t' {
        if options.show_tabs {
            stdout.write_all(b"^I")?;
        } else {
            stdout.write_all(&[byte])?;
        }
        return Ok(());
    }

    if !options.show_nonprinting {
        stdout.write_all(&[byte])?;
        return Ok(());
    }

    match byte {
        0..=31 => {
            stdout.write_all(b"^")?;
            stdout.write_all(&[byte + 64])?;
        }
        127 => stdout.write_all(b"^?")?,
        128..=159 => {
            stdout.write_all(b"M-^")?;
            stdout.write_all(&[byte - 64])?;
        }
        160..=254 => {
            stdout.write_all(b"M-")?;
            stdout.write_all(&[byte - 128])?;
        }
        255 => stdout.write_all(b"M-^?")?,
        _ => stdout.write_all(&[byte])?,
    }
    Ok(())
}

fn cat_error_message(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        io::ErrorKind::PermissionDenied if err.to_string() == "Is a directory" => {
            "Is a directory".to_string()
        }
        _ => err.to_string(),
    }
}

fn print_cat_help() {
    println!("Usage: cat [OPTION]... [FILE]...");
    println!("concatenate files and print on the standard output");
    println!();
    println!("With no FILE, or when FILE is -, read standard input.");
    println!();
    println!("  -A, --show-all          equivalent to -vET");
    println!("  -b, --number-nonblank   number nonempty output lines, overrides -n");
    println!("  -e                      equivalent to -vE");
    println!("  -E, --show-ends         display $ at end of each line");
    println!("  -n, --number            number all output lines");
    println!("  -s, --squeeze-blank     suppress repeated empty output lines");
    println!("  -t                      equivalent to -vT");
    println!("  -T, --show-tabs         display TAB characters as ^I");
    println!("  -u                      ignored, for POSIX compatibility");
    println!("  -v, --show-nonprinting  use ^ and M- notation, except for LFD and TAB");
    println!("  -h, --help              display this help and exit");
    println!("  -V, --version           output version information and exit");
    println!("      --                  stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_cat_options_matches_common_uutils_flags() {
        let (options, operands) =
            parse_cat_args(&["-benst".into(), "--".into(), "-literal".into()]).unwrap();

        assert_eq!(options.number, NumberMode::NonBlank);
        assert!(options.show_ends);
        assert!(options.show_nonprinting);
        assert!(options.squeeze_blank);
        assert!(options.show_tabs);
        assert_eq!(operands, vec!["-literal"]);
    }

    #[test]
    fn cat_reads_dash_prefixed_operand_after_separator() {
        let temp = unique_temp_dir("winuxsh-native-cat-dash");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("-p"), "payload").unwrap();

        let mut stdin = io::empty();
        let mut stdout = Vec::new();
        let code = execute_cat_with_io(
            &["--".into(), "-p".into()],
            &mut |arg| temp.join(arg),
            &mut stdin,
            &mut stdout,
        );

        assert_eq!(code, 0);
        assert_eq!(stdout, b"payload");
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn cat_numbers_nonblank_and_squeezes_blank_lines() {
        let temp = unique_temp_dir("winuxsh-native-cat-number");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("input.txt"), "a\n\n\nb\n").unwrap();

        let mut stdin = io::empty();
        let mut stdout = Vec::new();
        let code = execute_cat_with_io(
            &["-bs".into(), "input.txt".into()],
            &mut |arg| temp.join(arg),
            &mut stdin,
            &mut stdout,
        );

        assert_eq!(code, 0);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "     1\ta\n\n     2\tb\n"
        );
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn cat_show_all_renders_tabs_ends_and_controls() {
        let mut stdin = io::Cursor::new(b"a\t\x01\n".to_vec());
        let mut stdout = Vec::new();
        let code = execute_cat_with_io(
            &["-A".into()],
            &mut |_| PathBuf::new(),
            &mut stdin,
            &mut stdout,
        );

        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(stdout).unwrap(), "a^I^A$\n");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
    }
}
