use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use filetime::{set_file_times, FileTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinkMode {
    Copy,
    HardLink,
    SymbolicLink,
}

#[derive(Debug, Eq, PartialEq)]
struct CpOptions {
    recursive: bool,
    verbose: bool,
    force: bool,
    interactive: bool,
    no_clobber: bool,
    update: bool,
    backup: bool,
    suffix: String,
    target_directory: Option<String>,
    no_target_directory: bool,
    strip_trailing_slashes: bool,
    remove_destination: bool,
    attributes_only: bool,
    parents: bool,
    preserve_timestamps: bool,
    preserve_mode: bool,
    link_mode: LinkMode,
}

impl Default for CpOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            verbose: false,
            force: false,
            interactive: false,
            no_clobber: false,
            update: false,
            backup: false,
            suffix: "~".to_string(),
            target_directory: None,
            no_target_directory: false,
            strip_trailing_slashes: false,
            remove_destination: false,
            attributes_only: false,
            parents: false,
            preserve_timestamps: false,
            preserve_mode: false,
            link_mode: LinkMode::Copy,
        }
    }
}

#[derive(Clone, Debug)]
struct CopySource {
    display: String,
    path: PathBuf,
}

pub(crate) fn execute_cp<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    execute_cp_with_io(args, &mut resolve_path, &mut stdout, &mut stderr)
}

pub(crate) fn execute_cp_with_io<F, O, E>(
    args: &[String],
    mut resolve_path: F,
    stdout: &mut O,
    stderr: &mut E,
) -> i32
where
    F: FnMut(&str) -> PathBuf,
    O: Write,
    E: Write,
{
    let (options, operands) = match parse_cp_args(args, stdout, stderr) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    let (sources, destination) =
        match resolve_copy_operands(&options, operands, &mut resolve_path, stderr) {
            Ok(value) => value,
            Err(code) => return code,
        };

    let mut code = 0;
    let multiple_sources = sources.len() > 1 || options.target_directory.is_some();
    for source in sources {
        let target = if multiple_sources || (destination.is_dir() && !options.no_target_directory) {
            if options.parents {
                destination.join(parent_preserving_relative_path(&source.display))
            } else {
                destination.join(source_basename(&source.display, &source.path))
            }
        } else {
            destination.clone()
        };

        if let Err(err) = copy_item(&source.display, &source.path, &target, &options, stdout) {
            let _ = writeln!(
                stderr,
                "cp: cannot copy '{}': {}",
                source.display,
                cp_error(&err)
            );
            code = 1;
        }
    }

    code
}

fn resolve_copy_operands<F>(
    options: &CpOptions,
    operands: Vec<String>,
    resolve_path: &mut F,
    stderr: &mut impl Write,
) -> Result<(Vec<CopySource>, PathBuf), i32>
where
    F: FnMut(&str) -> PathBuf,
{
    if operands.is_empty() {
        let _ = writeln!(stderr, "cp: missing file operand");
        let _ = writeln!(stderr, "Try 'cp --help' for more information.");
        return Err(1);
    }

    let (source_operands, destination_operand) = if let Some(target) = &options.target_directory {
        (operands.as_slice(), target.as_str())
    } else {
        if operands.len() < 2 {
            let _ = writeln!(
                stderr,
                "cp: missing destination file operand after '{}'",
                operands[0]
            );
            let _ = writeln!(stderr, "Try 'cp --help' for more information.");
            return Err(1);
        }
        let split = operands.len() - 1;
        (&operands[..split], operands[split].as_str())
    };

    let destination = resolve_path(destination_operand);
    if source_operands.len() > 1 || options.target_directory.is_some() {
        match fs::metadata(&destination) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                let _ = writeln!(
                    stderr,
                    "cp: target '{}' is not a directory",
                    destination_operand
                );
                return Err(1);
            }
            Err(err) => {
                let _ = writeln!(
                    stderr,
                    "cp: target directory '{}': {}",
                    destination_operand,
                    cp_error(&err)
                );
                return Err(1);
            }
        }
    }

    let sources = source_operands
        .iter()
        .map(|operand| {
            let display = if options.strip_trailing_slashes {
                strip_trailing_separators(operand)
            } else {
                operand.clone()
            };
            let path = resolve_path(&display);
            CopySource { display, path }
        })
        .collect();

    Ok((sources, destination))
}

fn parse_cp_args<O, E>(
    args: &[String],
    stdout: &mut O,
    stderr: &mut E,
) -> Result<(CpOptions, Vec<String>), i32>
where
    O: Write,
    E: Write,
{
    let mut options = CpOptions::default();
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
            print_cp_help(stdout);
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            let _ = writeln!(stdout, "cp (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg.starts_with("--") {
            parse_cp_long_option(arg, args, &mut index, &mut options, stderr)?;
            continue;
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            parse_cp_short_options(arg, args, &mut index, &mut options, stderr)?;
            continue;
        }
        operands.push(arg.clone());
        index += 1;
    }

    Ok((options, operands))
}

fn parse_cp_long_option(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut CpOptions,
    stderr: &mut impl Write,
) -> Result<(), i32> {
    let (name, value) = split_long_option(arg);
    match name {
        "archive" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.recursive = true;
            options.preserve_timestamps = true;
            options.preserve_mode = true;
            *index += 1;
        }
        "backup" => {
            options.backup = true;
            if let Some(value) = value {
                let _ = value;
            }
            *index += 1;
        }
        "force" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.force = true;
            options.no_clobber = false;
            *index += 1;
        }
        "interactive" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.interactive = true;
            options.no_clobber = false;
            *index += 1;
        }
        "no-clobber" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.no_clobber = true;
            options.interactive = false;
            *index += 1;
        }
        "recursive" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.recursive = true;
            *index += 1;
        }
        "link" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.link_mode = LinkMode::HardLink;
            *index += 1;
        }
        "symbolic-link" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.link_mode = LinkMode::SymbolicLink;
            *index += 1;
        }
        "suffix" => {
            options.suffix = take_long_value("cp", arg, value, args, index, stderr)?;
        }
        "target-directory" => {
            options.target_directory =
                Some(take_long_value("cp", arg, value, args, index, stderr)?);
        }
        "no-target-directory" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.no_target_directory = true;
            *index += 1;
        }
        "strip-trailing-slashes" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.strip_trailing_slashes = true;
            *index += 1;
        }
        "update" => {
            options.update = true;
            *index += 1;
        }
        "verbose" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.verbose = true;
            *index += 1;
        }
        "remove-destination" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.remove_destination = true;
            *index += 1;
        }
        "attributes-only" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.attributes_only = true;
            *index += 1;
        }
        "parents" | "parent" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.parents = true;
            *index += 1;
        }
        "dereference" => {
            reject_long_value("cp", arg, value, stderr)?;
            *index += 1;
        }
        "no-dereference" => {
            reject_long_value("cp", arg, value, stderr)?;
            *index += 1;
        }
        "debug" => {
            reject_long_value("cp", arg, value, stderr)?;
            options.verbose = true;
            *index += 1;
        }
        "one-file-system" | "copy-contents" | "context" | "progress-bar" => {
            *index += 1;
        }
        "preserve" => {
            options.preserve_timestamps = true;
            options.preserve_mode = true;
            *index += 1;
        }
        "no-preserve" | "sparse" | "reflink" => {
            *index += 1;
        }
        _ => {
            let _ = writeln!(stderr, "cp: unrecognized option '{}'", arg);
            let _ = writeln!(stderr, "Try 'cp --help' for more information.");
            return Err(1);
        }
    }
    Ok(())
}

fn parse_cp_short_options(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut CpOptions,
    stderr: &mut impl Write,
) -> Result<(), i32> {
    let chars: Vec<char> = arg.chars().collect();
    let mut pos = 1;
    while pos < chars.len() {
        match chars[pos] {
            'a' => {
                options.recursive = true;
                options.preserve_timestamps = true;
                options.preserve_mode = true;
            }
            'b' => options.backup = true,
            'd' | 'H' | 'L' | 'P' | 'x' | 'Z' | 'c' | 'g' => {}
            'f' => {
                options.force = true;
                options.no_clobber = false;
            }
            'i' => {
                options.interactive = true;
                options.no_clobber = false;
            }
            'l' => options.link_mode = LinkMode::HardLink,
            'n' => {
                options.no_clobber = true;
                options.interactive = false;
            }
            'p' => {
                options.preserve_timestamps = true;
                options.preserve_mode = true;
            }
            'R' | 'r' => options.recursive = true,
            's' => options.link_mode = LinkMode::SymbolicLink,
            'T' => options.no_target_directory = true,
            'u' => options.update = true,
            'v' => options.verbose = true,
            'S' | 't' => {
                let option = chars[pos];
                let value = if pos + 1 < chars.len() {
                    chars[pos + 1..].iter().collect()
                } else {
                    *index += 1;
                    let Some(value) = args.get(*index) else {
                        let _ = writeln!(stderr, "cp: option '-{}' requires an argument", option);
                        let _ = writeln!(stderr, "Try 'cp --help' for more information.");
                        return Err(1);
                    };
                    value.clone()
                };
                if option == 'S' {
                    options.suffix = value;
                } else {
                    options.target_directory = Some(value);
                }
                *index += 1;
                return Ok(());
            }
            other => {
                let _ = writeln!(stderr, "cp: invalid option -- '{}'", other);
                let _ = writeln!(stderr, "Try 'cp --help' for more information.");
                return Err(1);
            }
        }
        pos += 1;
    }
    *index += 1;
    Ok(())
}

fn copy_item(
    display: &str,
    source: &Path,
    destination: &Path,
    options: &CpOptions,
    stdout: &mut impl Write,
) -> io::Result<()> {
    let metadata = fs::metadata(source)?;
    if metadata.is_dir() {
        if !options.recursive {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "omitting directory",
            ));
        }
        copy_dir(display, source, destination, options, stdout)
    } else {
        copy_file_like(display, source, destination, &metadata, options, stdout)
    }
}

fn copy_dir(
    display: &str,
    source: &Path,
    destination: &Path,
    options: &CpOptions,
    stdout: &mut impl Write,
) -> io::Result<()> {
    if destination.exists() && !destination.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination exists and is not a directory",
        ));
    }
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }

    if options.verbose {
        let _ = writeln!(stdout, "'{}' -> '{}'", display, destination.display());
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let child_source = entry.path();
        let child_name = entry.file_name();
        let child_display = format!(
            "{}/{}",
            display.trim_end_matches(['/', '\\']),
            child_name.to_string_lossy()
        );
        let child_destination = destination.join(child_name);
        copy_item(
            &child_display,
            &child_source,
            &child_destination,
            options,
            stdout,
        )?;
    }

    if options.preserve_mode {
        let permissions = fs::metadata(source)?.permissions();
        let _ = fs::set_permissions(destination, permissions);
    }
    if options.preserve_timestamps {
        preserve_times(source, destination)?;
    }
    Ok(())
}

fn copy_file_like(
    display: &str,
    source: &Path,
    destination: &Path,
    metadata: &fs::Metadata,
    options: &CpOptions,
    stdout: &mut impl Write,
) -> io::Result<()> {
    if options.update && destination.exists() {
        if let (Ok(source_time), Ok(dest_time)) =
            (metadata.modified(), fs::metadata(destination)?.modified())
        {
            if dest_time >= source_time {
                return Ok(());
            }
        }
    }

    if destination.exists() {
        if options.no_clobber {
            return Ok(());
        }
        if options.interactive
            && !prompt_yes(&format!("cp: overwrite '{}'? ", destination.display()))
        {
            return Ok(());
        }
    }

    prepare_destination(display, destination, options)?;

    match options.link_mode {
        LinkMode::Copy => {
            if options.attributes_only {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(false)
                    .open(destination)?;
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, destination)?;
            }
        }
        LinkMode::HardLink => {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::hard_link(source, destination)?;
        }
        LinkMode::SymbolicLink => {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            create_symlink(source, destination, metadata.is_dir())?;
        }
    }

    if options.preserve_mode || options.attributes_only {
        let _ = fs::set_permissions(destination, metadata.permissions());
    }
    if options.preserve_timestamps {
        preserve_times(source, destination)?;
    }
    if options.verbose {
        let _ = writeln!(stdout, "'{}' -> '{}'", display, destination.display());
    }
    Ok(())
}

fn prepare_destination(display: &str, destination: &Path, options: &CpOptions) -> io::Result<()> {
    if !destination.exists() {
        return Ok(());
    }
    if options.backup {
        create_backup(destination, &options.suffix)?;
    }
    if options.remove_destination
        || options.force
        || matches!(
            options.link_mode,
            LinkMode::HardLink | LinkMode::SymbolicLink
        )
    {
        remove_destination_path(destination)?;
    } else if destination.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("cannot overwrite directory with '{}'", display),
        ));
    }
    Ok(())
}

fn create_backup(path: &Path, suffix: &str) -> io::Result<()> {
    let backup = PathBuf::from(format!("{}{}", path.display(), suffix));
    if backup.exists() {
        remove_destination_path(&backup)?;
    }
    fs::copy(path, backup)?;
    Ok(())
}

fn remove_destination_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn preserve_times(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::metadata(source)?;
    let atime = FileTime::from_last_access_time(&metadata);
    let mtime = FileTime::from_last_modification_time(&metadata);
    set_file_times(destination, atime, mtime)
}

#[cfg(windows)]
fn create_symlink(source: &Path, destination: &Path, is_dir: bool) -> io::Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(source, destination)
    } else {
        std::os::windows::fs::symlink_file(source, destination)
    }
}

#[cfg(unix)]
fn create_symlink(source: &Path, destination: &Path, _is_dir: bool) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_source: &Path, _destination: &Path, _is_dir: bool) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "symbolic links are not supported on this platform",
    ))
}

fn prompt_yes(prompt: &str) -> bool {
    if !io::stdin().is_terminal() {
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

fn parent_preserving_relative_path(display: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for component in Path::new(display).components() {
        match component {
            Component::Normal(value) => path.push(value),
            Component::ParentDir => path.push(".."),
            Component::CurDir => {}
            Component::Prefix(prefix) => {
                path.push(prefix.as_os_str().to_string_lossy().replace(':', ""))
            }
            Component::RootDir => {}
        }
    }
    if path.as_os_str().is_empty() {
        PathBuf::from(source_basename(display, Path::new(display)))
    } else {
        path
    }
}

fn source_basename(display: &str, path: &Path) -> String {
    let slash_display = display.replace('\\', "/");
    if slash_display.ends_with("/.") {
        return ".".to_string();
    }
    let cleaned = strip_trailing_separators(display);
    Path::new(&cleaned)
        .file_name()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| cleaned)
}

fn strip_trailing_separators(value: &str) -> String {
    let trimmed = value.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        value.to_string()
    } else {
        trimmed.to_string()
    }
}

fn split_long_option(arg: &str) -> (&str, Option<&str>) {
    arg.strip_prefix("--")
        .unwrap_or(arg)
        .split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or_else(|| (arg.strip_prefix("--").unwrap_or(arg), None))
}

fn reject_long_value(
    command: &str,
    arg: &str,
    value: Option<&str>,
    stderr: &mut impl Write,
) -> Result<(), i32> {
    if value.is_some() {
        let _ = writeln!(
            stderr,
            "{}: option '{}' doesn't allow an argument",
            command, arg
        );
        let _ = writeln!(stderr, "Try '{} --help' for more information.", command);
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
    stderr: &mut impl Write,
) -> Result<String, i32> {
    if let Some(value) = value {
        *index += 1;
        return Ok(value.to_string());
    }
    *index += 1;
    let Some(value) = args.get(*index) else {
        let _ = writeln!(stderr, "{}: option '{}' requires an argument", command, arg);
        let _ = writeln!(stderr, "Try '{} --help' for more information.", command);
        return Err(1);
    };
    *index += 1;
    Ok(value.clone())
}

fn cp_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        io::ErrorKind::AlreadyExists => err.to_string(),
        io::ErrorKind::InvalidInput => err.to_string(),
        io::ErrorKind::Interrupted => err.to_string(),
        _ => err.to_string(),
    }
}

fn print_cp_help(stdout: &mut impl Write) {
    let _ = write!(
        stdout,
        "Usage: cp [OPTION]... SOURCE DEST\n  or:  cp [OPTION]... SOURCE... DIRECTORY\ncopy files and directories\n\n  -a, --archive                 same as -dR --preserve=all\n  -b, --backup                  make a backup of each existing destination file\n  -f, --force                   remove existing destination and try again\n  -i, --interactive             prompt before overwrite\n  -l, --link                    hard link files instead of copying\n  -n, --no-clobber              do not overwrite an existing file\n  -p, --preserve                preserve mode and timestamps\n  -R, -r, --recursive           copy directories recursively\n  -s, --symbolic-link           make symbolic links instead of copying\n  -S, --suffix SUFFIX           override the usual backup suffix\n  -t, --target-directory DIR    copy all SOURCE arguments into DIR\n  -T, --no-target-directory     treat DEST as a normal file\n      --strip-trailing-slashes  remove trailing slashes from SOURCE arguments\n  -u, --update                  copy only when SOURCE is newer\n  -v, --verbose                 explain what is being done\n      --remove-destination      remove existing destination before copying\n      --attributes-only         copy attributes without file data\n      --parents                 use full source file name under DIRECTORY\n  -H, -L, -P, -d, -x, -Z, -c    accepted for compatibility\n  -h, --help                    display this help and exit\n  -V, --version                 output version information and exit\n      --                        stop option parsing\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cp_common_options() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let (options, operands) = parse_cp_args(
            &[
                "-av".into(),
                "--backup".into(),
                "-S".into(),
                ".bak".into(),
                "--".into(),
                "-src".into(),
                "dst".into(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert!(options.recursive);
        assert!(options.preserve_timestamps);
        assert!(options.verbose);
        assert!(options.backup);
        assert_eq!(options.suffix, ".bak");
        assert_eq!(operands, vec!["-src", "dst"]);
    }

    #[test]
    fn parent_preserving_path_drops_root_prefix() {
        assert_eq!(
            parent_preserving_relative_path("/tmp/source.txt"),
            PathBuf::from("tmp").join("source.txt")
        );
    }
}
