pub(crate) fn execute_pwd(args: &[String], logical_pwd: &str) -> i32 {
    let mut physical = false;
    for arg in args {
        if arg == "--" {
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        for option in arg[1..].chars() {
            match option {
                'L' => physical = false,
                'P' => physical = true,
                other => {
                    eprintln!("winuxsh: pwd: -{}: invalid option", other);
                    eprintln!("pwd: usage: pwd [-LP]");
                    return 2;
                }
            }
        }
    }

    let directory = if physical {
        std::env::current_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| logical_pwd.to_string())
    } else {
        logical_pwd.to_string()
    };

    println!("{}", display_path(&directory));
    0
}

fn display_path(value: &str) -> String {
    if cfg!(windows) {
        shell_path_to_host_path(value).replace('/', "\\")
    } else {
        value.to_string()
    }
}

fn shell_path_to_host_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    if cfg!(windows) {
        let bytes = normalized.as_bytes();
        if bytes.len() == 2 && bytes[0] == b'/' && (bytes[1] as char).is_ascii_alphabetic() {
            let drive = (bytes[1] as char).to_ascii_uppercase();
            return format!("{}:/", drive);
        }
        if bytes.len() >= 3
            && bytes[0] == b'/'
            && (bytes[1] as char).is_ascii_alphabetic()
            && bytes[2] == b'/'
        {
            let drive = (bytes[1] as char).to_ascii_uppercase();
            return format!("{}:{}", drive, &normalized[2..]);
        }
    }
    value.to_string()
}
