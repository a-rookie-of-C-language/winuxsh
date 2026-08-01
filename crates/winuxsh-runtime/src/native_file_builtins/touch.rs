use std::fs::{self, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use filetime::{set_file_times, FileTime};

#[derive(Debug, Default, Eq, PartialEq)]
struct TouchOptions {
    no_create: bool,
    access_only: bool,
    modify_only: bool,
    no_dereference: bool,
    reference: Option<String>,
    timestamp: Option<FileTime>,
}

pub(crate) fn execute_touch<F>(args: &[String], mut resolve_path: F) -> i32
where
    F: FnMut(&str) -> PathBuf,
{
    let (options, operands) = match parse_touch_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };

    if operands.is_empty() {
        eprintln!("touch: missing file operand");
        eprintln!("Try 'touch --help' for more information.");
        return 1;
    }

    let reference_times = match options.reference.as_ref() {
        Some(reference) => {
            let path = resolve_path(reference);
            match fs::metadata(&path) {
                Ok(metadata) => Some((
                    FileTime::from_last_access_time(&metadata),
                    FileTime::from_last_modification_time(&metadata),
                )),
                Err(err) => {
                    eprintln!(
                        "touch: failed to get attributes of '{}': {}",
                        reference,
                        touch_error(&err)
                    );
                    return 1;
                }
            }
        }
        None => None,
    };

    let mut code = 0;
    for operand in operands {
        let path = resolve_path(&operand);
        if let Err(err) = touch_one(&path, &options, reference_times) {
            eprintln!("touch: cannot touch '{}': {}", operand, touch_error(&err));
            code = 1;
        }
    }
    code
}

fn touch_one(
    path: &PathBuf,
    options: &TouchOptions,
    reference_times: Option<(FileTime, FileTime)>,
) -> io::Result<()> {
    let mut metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound && options.no_create => return Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(path)?;
            fs::metadata(path)?
        }
        Err(err) => return Err(err),
    };

    if metadata.is_dir() {
        metadata = fs::metadata(path)?;
    }

    let now = FileTime::from_system_time(SystemTime::now());
    let (candidate_atime, candidate_mtime) = reference_times.unwrap_or_else(|| {
        options
            .timestamp
            .map(|time| (time, time))
            .unwrap_or((now, now))
    });
    let set_access = options.access_only || !options.modify_only;
    let set_modify = options.modify_only || !options.access_only;
    let atime = if set_access {
        candidate_atime
    } else {
        FileTime::from_last_access_time(&metadata)
    };
    let mtime = if set_modify {
        candidate_mtime
    } else {
        FileTime::from_last_modification_time(&metadata)
    };

    let _ = options.no_dereference;
    set_file_times(path, atime, mtime)
}

fn parse_touch_args(args: &[String]) -> Result<(TouchOptions, Vec<String>), i32> {
    let mut options = TouchOptions::default();
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
            print_touch_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("touch (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && arg.starts_with("--") {
            parse_touch_long_option(arg, args, &mut index, &mut options)?;
            continue;
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            parse_touch_short_options(arg, args, &mut index, &mut options)?;
            continue;
        }
        operands.push(arg.clone());
        index += 1;
    }

    Ok((options, operands))
}

fn parse_touch_long_option(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut TouchOptions,
) -> Result<(), i32> {
    let (name, value) = split_long_option(arg);
    match name {
        "no-create" => {
            reject_long_value("touch", arg, value)?;
            options.no_create = true;
            *index += 1;
        }
        "no-dereference" => {
            reject_long_value("touch", arg, value)?;
            options.no_dereference = true;
            *index += 1;
        }
        "date" => {
            let value = take_long_value("touch", arg, value, args, index)?;
            options.timestamp = Some(parse_date_time(&value)?);
        }
        "reference" => {
            options.reference = Some(take_long_value("touch", arg, value, args, index)?);
        }
        "time" => {
            let value = take_long_value("touch", arg, value, args, index)?;
            apply_time_selector("touch", &value, options)?;
        }
        _ => {
            eprintln!("touch: unrecognized option '{}'", arg);
            eprintln!("Try 'touch --help' for more information.");
            return Err(1);
        }
    }
    Ok(())
}

fn parse_touch_short_options(
    arg: &str,
    args: &[String],
    index: &mut usize,
    options: &mut TouchOptions,
) -> Result<(), i32> {
    let chars: Vec<char> = arg.chars().collect();
    let mut pos = 1;
    while pos < chars.len() {
        match chars[pos] {
            'a' => options.access_only = true,
            'c' => options.no_create = true,
            'f' => {}
            'm' => options.modify_only = true,
            'd' | 'r' | 't' => {
                let option = chars[pos];
                let value = if pos + 1 < chars.len() {
                    chars[pos + 1..].iter().collect()
                } else {
                    *index += 1;
                    let Some(value) = args.get(*index) else {
                        eprintln!("touch: option '-{}' requires an argument", option);
                        eprintln!("Try 'touch --help' for more information.");
                        return Err(1);
                    };
                    value.clone()
                };
                match option {
                    'd' => options.timestamp = Some(parse_date_time(&value)?),
                    'r' => options.reference = Some(value),
                    't' => options.timestamp = Some(parse_touch_timestamp(&value)?),
                    _ => unreachable!(),
                }
                *index += 1;
                return Ok(());
            }
            other => {
                eprintln!("touch: invalid option -- '{}'", other);
                eprintln!("Try 'touch --help' for more information.");
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

fn apply_time_selector(command: &str, value: &str, options: &mut TouchOptions) -> Result<(), i32> {
    match value {
        "access" | "atime" | "use" => {
            options.access_only = true;
            options.modify_only = false;
        }
        "modify" | "mtime" => {
            options.modify_only = true;
            options.access_only = false;
        }
        _ => {
            eprintln!("{}: invalid argument '{}' for '--time'", command, value);
            return Err(1);
        }
    }
    Ok(())
}

fn parse_date_time(value: &str) -> Result<FileTime, i32> {
    if let Some(seconds) = value.strip_prefix('@') {
        let seconds = seconds.parse::<i64>().map_err(|_| {
            eprintln!("touch: invalid date format '{}'", value);
            1
        })?;
        return Ok(FileTime::from_unix_time(seconds, 0));
    }

    if let Ok(time) = parse_touch_timestamp(value) {
        return Ok(time);
    }

    let normalized = value.replace('T', " ");
    let mut parts = normalized.split_whitespace();
    let Some(date) = parts.next() else {
        eprintln!("touch: invalid date format '{}'", value);
        return Err(1);
    };
    let time = parts.next().unwrap_or("00:00:00");
    if parts.next().is_some() {
        eprintln!("touch: invalid date format '{}'", value);
        return Err(1);
    }

    let date_parts: Vec<_> = date.split('-').collect();
    if date_parts.len() != 3 {
        eprintln!("touch: invalid date format '{}'", value);
        return Err(1);
    }
    let year = parse_i32(date_parts[0], value)?;
    let month = parse_u32(date_parts[1], value)?;
    let day = parse_u32(date_parts[2], value)?;

    let time_parts: Vec<_> = time.split(':').collect();
    if !(2..=3).contains(&time_parts.len()) {
        eprintln!("touch: invalid date format '{}'", value);
        return Err(1);
    }
    let hour = parse_u32(time_parts[0], value)?;
    let minute = parse_u32(time_parts[1], value)?;
    let second = if time_parts.len() == 3 {
        parse_u32(time_parts[2], value)?
    } else {
        0
    };
    file_time_from_parts(year, month, day, hour, minute, second, value)
}

fn parse_touch_timestamp(value: &str) -> Result<FileTime, i32> {
    let (main, second) = match value.split_once('.') {
        Some((main, second)) => (main, parse_u32(second, value)?),
        None => (value, 0),
    };
    if !main.chars().all(|ch| ch.is_ascii_digit()) {
        eprintln!("touch: invalid date format '{}'", value);
        return Err(1);
    }

    let (year, offset) = match main.len() {
        8 => (current_year_utc(), 0),
        10 => {
            let yy = parse_i32(&main[0..2], value)?;
            (if yy >= 69 { 1900 + yy } else { 2000 + yy }, 2)
        }
        12 => (parse_i32(&main[0..4], value)?, 4),
        _ => {
            eprintln!("touch: invalid date format '{}'", value);
            return Err(1);
        }
    };

    let month = parse_u32(&main[offset..offset + 2], value)?;
    let day = parse_u32(&main[offset + 2..offset + 4], value)?;
    let hour = parse_u32(&main[offset + 4..offset + 6], value)?;
    let minute = parse_u32(&main[offset + 6..offset + 8], value)?;
    file_time_from_parts(year, month, day, hour, minute, second, value)
}

fn file_time_from_parts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    original: &str,
) -> Result<FileTime, i32> {
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 60
    {
        eprintln!("touch: invalid date format '{}'", original);
        return Err(1);
    }
    let days = days_from_civil(year, month as i32, day as i32);
    let seconds = days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64;
    Ok(FileTime::from_unix_time(seconds, 0))
}

fn parse_i32(value: &str, original: &str) -> Result<i32, i32> {
    value.parse::<i32>().map_err(|_| {
        eprintln!("touch: invalid date format '{}'", original);
        1
    })
}

fn parse_u32(value: &str, original: &str) -> Result<u32, i32> {
    value.parse::<u32>().map_err(|_| {
        eprintln!("touch: invalid date format '{}'", original);
        1
    })
}

fn current_year_utc() -> i32 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(seconds.div_euclid(86_400)).0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as i64
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

fn touch_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        _ => err.to_string(),
    }
}

fn print_touch_help() {
    println!("Usage: touch [OPTION]... FILE...");
    println!("Update the access and modification times of each FILE to the current time.");
    println!();
    println!("  -a                         change only the access time");
    println!("  -c, --no-create            do not create any files");
    println!("  -d, --date STRING          parse STRING and use it instead of current time");
    println!("  -f                         ignored");
    println!("      --no-dereference       accepted for compatibility");
    println!("  -m                         change only the modification time");
    println!("  -r, --reference FILE       use this file's times instead of current time");
    println!("  -t STAMP                   use [[CC]YY]MMDDhhmm[.ss] instead of current time");
    println!("      --time WORD            change access/atime/use or modify/mtime");
    println!("  -h, --help                 display this help and exit");
    println!("  -V, --version              output version information and exit");
    println!("      --                     stop option parsing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_touch_timestamp() {
        assert!(parse_touch_timestamp("202001020304.05").is_ok());
        assert!(parse_touch_timestamp("01020304").is_ok());
        assert!(parse_touch_timestamp("bad").is_err());
    }

    #[test]
    fn parses_touch_common_options() {
        let (options, operands) = parse_touch_args(&[
            "-am".into(),
            "-t".into(),
            "202001010000".into(),
            "--".into(),
            "-file".into(),
        ])
        .unwrap();

        assert!(options.access_only);
        assert!(options.modify_only);
        assert!(options.timestamp.is_some());
        assert_eq!(operands, vec!["-file"]);
    }
}
