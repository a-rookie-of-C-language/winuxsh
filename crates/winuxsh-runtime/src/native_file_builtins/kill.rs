// Winuxsh-native `kill`, aligned with winuxcmd's small Windows-facing option
// surface while fixing the documented `-SIG`/`-SIGNAME` spellings.

use std::io;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SignalSpec {
    number: i32,
    name: &'static str,
    description: &'static str,
}

const SIGNALS: &[SignalSpec] = &[
    SignalSpec {
        number: 1,
        name: "HUP",
        description: "Hangup",
    },
    SignalSpec {
        number: 2,
        name: "INT",
        description: "Interrupt",
    },
    SignalSpec {
        number: 3,
        name: "QUIT",
        description: "Quit",
    },
    SignalSpec {
        number: 6,
        name: "ABRT",
        description: "Abort",
    },
    SignalSpec {
        number: 9,
        name: "KILL",
        description: "Kill (cannot be caught or ignored)",
    },
    SignalSpec {
        number: 11,
        name: "SEGV",
        description: "Segmentation fault",
    },
    SignalSpec {
        number: 13,
        name: "PIPE",
        description: "Broken pipe",
    },
    SignalSpec {
        number: 14,
        name: "ALRM",
        description: "Alarm clock",
    },
    SignalSpec {
        number: 15,
        name: "TERM",
        description: "Termination",
    },
    SignalSpec {
        number: 17,
        name: "STOP",
        description: "Stop (cannot be caught or ignored)",
    },
    SignalSpec {
        number: 18,
        name: "TSTP",
        description: "Terminal stop",
    },
    SignalSpec {
        number: 19,
        name: "CONT",
        description: "Continue",
    },
    SignalSpec {
        number: 20,
        name: "CHLD",
        description: "Child status changed",
    },
    SignalSpec {
        number: 21,
        name: "TTIN",
        description: "Background read from tty",
    },
    SignalSpec {
        number: 22,
        name: "TTOU",
        description: "Background write to tty",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KillMode {
    Kill,
    List,
    Table,
}

#[derive(Debug, Eq, PartialEq)]
struct KillOptions {
    mode: KillMode,
    signal: i32,
    pids: Vec<String>,
}

impl Default for KillOptions {
    fn default() -> Self {
        Self {
            mode: KillMode::Kill,
            signal: 15,
            pids: Vec::new(),
        }
    }
}

pub(crate) fn execute_kill(args: &[String]) -> i32 {
    let options = match parse_kill_args(args) {
        Ok(options) => options,
        Err(code) => return code,
    };

    match options.mode {
        KillMode::List => {
            print_signal_list();
            0
        }
        KillMode::Table => {
            print_signal_table();
            0
        }
        KillMode::Kill => execute_kill_pids(&options),
    }
}

fn execute_kill_pids(options: &KillOptions) -> i32 {
    if options.pids.is_empty() {
        eprintln!("kill: no process ID specified");
        return 1;
    }

    let mut code = 0;
    for pid_arg in &options.pids {
        let pid = match pid_arg.parse::<i32>() {
            Ok(pid) if pid > 0 => pid as u32,
            _ => {
                eprintln!("kill: invalid PID: {}", pid_arg);
                code = 1;
                continue;
            }
        };

        if let Err(err) = send_signal(pid, options.signal) {
            eprintln!("kill: ({}) - {}", pid, kill_error_message(&err));
            code = 1;
        }
    }

    code
}

fn parse_kill_args(args: &[String]) -> Result<KillOptions, i32> {
    let mut options = KillOptions::default();
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
            print_kill_help();
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-V" | "--version") {
            println!("kill (winuxsh native) {}", env!("CARGO_PKG_VERSION"));
            return Err(0);
        }
        if parse_options && matches!(arg.as_str(), "-l" | "--list") {
            options.mode = KillMode::List;
            index += 1;
            continue;
        }
        if parse_options && matches!(arg.as_str(), "-t" | "-L" | "--table") {
            options.mode = KillMode::Table;
            index += 1;
            continue;
        }
        if parse_options && arg.starts_with("--list=") {
            options.mode = KillMode::List;
            index += 1;
            continue;
        }
        if parse_options && arg.starts_with("--table=") {
            options.mode = KillMode::Table;
            index += 1;
            continue;
        }
        if parse_options && matches!(arg.as_str(), "-s" | "-n" | "--signal") {
            let Some(value) = args.get(index + 1) else {
                eprintln!("kill: option '{}' requires an argument", arg);
                eprintln!("Try 'kill --help' for more information.");
                return Err(1);
            };
            options.signal = parse_signal(value)?;
            index += 2;
            continue;
        }
        if parse_options && arg.starts_with("--signal=") {
            let value = arg.trim_start_matches("--signal=");
            if value.is_empty() {
                eprintln!("kill: option '--signal' requires an argument");
                eprintln!("Try 'kill --help' for more information.");
                return Err(1);
            }
            options.signal = parse_signal(value)?;
            index += 1;
            continue;
        }
        if parse_options && arg.starts_with('-') && arg != "-" {
            let signal = &arg[1..];
            if signal.chars().all(|ch| ch.is_ascii_digit())
                || signal.chars().any(|ch| ch.is_ascii_uppercase())
            {
                options.signal = parse_signal(signal)?;
                index += 1;
                continue;
            }

            let bad = arg.chars().nth(1).unwrap_or('-');
            eprintln!("kill: unrecognized option '-{}'", bad);
            eprintln!("Try 'kill --help' for more information.");
            return Err(1);
        }

        options.pids.push(arg.clone());
        index += 1;
    }

    Ok(options)
}

fn parse_signal(value: &str) -> Result<i32, i32> {
    if let Ok(number) = value.parse::<i32>() {
        if number == 0 || SIGNALS.iter().any(|signal| signal.number == number) {
            return Ok(number);
        }
    }

    let name = value
        .strip_prefix("SIG")
        .unwrap_or(value)
        .to_ascii_uppercase();
    if let Some(signal) = SIGNALS.iter().find(|signal| signal.name == name) {
        return Ok(signal.number);
    }

    eprintln!("kill: unknown signal: {}", value);
    Err(1)
}

fn print_signal_list() {
    for signal in SIGNALS {
        print!("{} ", signal.name);
    }
    println!();
}

fn print_signal_table() {
    println!("Signal  Name    Description");
    println!("------  ------  -----------");
    for signal in SIGNALS {
        println!(
            "{:<7} {:<7} {}",
            signal.number, signal.name, signal.description
        );
    }
}

fn print_kill_help() {
    println!("Usage: kill [OPTION]... [PID]...");
    println!("send a signal to a process");
    println!();
    println!("Send signals to processes, or list signals.");
    println!();
    println!("The default signal for kill is TERM. Use -l or -L to list available signals.");
    println!("Alternate signals may be specified as -9, -15, -SIGKILL, -KILL,");
    println!("or with -s/--signal.");
    println!();
    println!("  -s, --signal SIGNAL  specify the signal to send");
    println!("  -l, --list           list signal names");
    println!("  -L, --table          list signal names in a table");
    println!("  -h, --help           display this help and exit");
    println!("  -V, --version        output version information and exit");
    println!("      --               stop option parsing");
}

fn kill_error_message(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "no such process".to_string(),
        io::ErrorKind::PermissionDenied => "operation not permitted".to_string(),
        _ => err.to_string(),
    }
}

#[cfg(windows)]
fn send_signal(pid: u32, signal: i32) -> io::Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
    };

    const WAIT_TIMEOUT_MS: u32 = 200;

    let access = if signal == 0 {
        PROCESS_QUERY_LIMITED_INFORMATION
    } else {
        PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE
    };

    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() {
        return Err(windows_last_error_as_io());
    }

    let result = if signal == 0 {
        Ok(())
    } else if signal == 15 && try_console_break(pid) {
        let wait = unsafe { WaitForSingleObject(handle, WAIT_TIMEOUT_MS) };
        if wait == WAIT_OBJECT_0 {
            Ok(())
        } else {
            terminate_process(handle)
        }
    } else {
        terminate_process(handle)
    };

    unsafe {
        CloseHandle(handle);
    }
    result
}

#[cfg(windows)]
fn terminate_process(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<()> {
    use windows_sys::Win32::System::Threading::TerminateProcess;

    if unsafe { TerminateProcess(handle, 1) } == 0 {
        Err(windows_last_error_as_io())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn try_console_break(pid: u32) -> bool {
    use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};

    unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) != 0 }
}

#[cfg(windows)]
fn windows_last_error_as_io() -> io::Error {
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER};

    let raw = unsafe { windows_sys::Win32::Foundation::GetLastError() } as i32;
    match raw as u32 {
        ERROR_INVALID_PARAMETER => io::Error::new(io::ErrorKind::NotFound, "no such process"),
        ERROR_ACCESS_DENIED => {
            io::Error::new(io::ErrorKind::PermissionDenied, "operation not permitted")
        }
        _ => io::Error::from_raw_os_error(raw),
    }
}

#[cfg(not(windows))]
fn send_signal(pid: u32, signal: i32) -> io::Result<()> {
    let status = std::process::Command::new("kill")
        .arg(format!("-{}", signal))
        .arg(pid.to_string())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "kill failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kill_winuxcmd_options() {
        let options = parse_kill_args(&["--signal".into(), "KILL".into(), "123".into()]).unwrap();
        assert_eq!(options.mode, KillMode::Kill);
        assert_eq!(options.signal, 9);
        assert_eq!(options.pids, vec!["123"]);

        let options = parse_kill_args(&["-15".into(), "123".into()]).unwrap();
        assert_eq!(options.signal, 15);

        let options = parse_kill_args(&["-SIGKILL".into(), "123".into()]).unwrap();
        assert_eq!(options.signal, 9);
    }

    #[test]
    fn parse_kill_list_and_table_modes() {
        let list = parse_kill_args(&["-l".into(), "TERM".into()]).unwrap();
        assert_eq!(list.mode, KillMode::List);
        assert_eq!(list.pids, vec!["TERM"]);

        let table = parse_kill_args(&["--table".into()]).unwrap();
        assert_eq!(table.mode, KillMode::Table);
    }

    #[test]
    fn parse_kill_rejects_unknown_signal() {
        assert_eq!(
            parse_kill_args(&["--signal".into(), "NOPE".into(), "123".into()]),
            Err(1)
        );
    }
}
