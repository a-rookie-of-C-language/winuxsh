use std::path::{Path, PathBuf};

const EXECUTABLE_EXTENSIONS: &[&str] = &[".exe", ".bat", ".cmd", ".ps1", ".com"];

pub fn clean_command_name(cmd: &str) -> String {
    cmd.trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace())
        .to_string()
}

pub fn find_command(cmd: &str, current_dir: &Path, path_env: Option<&str>) -> Option<PathBuf> {
    let clean_cmd = clean_command_name(cmd);
    if clean_cmd.is_empty() {
        return None;
    }

    if let Some(path) = find_in_dir(current_dir, &clean_cmd) {
        return Some(path);
    }

    if clean_cmd.contains('\\') || clean_cmd.contains('/') {
        let path = PathBuf::from(&clean_cmd);
        let resolved = if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        };
        return resolved.exists().then_some(resolved);
    }

    let path_env = path_env?;
    for dir in std::env::split_paths(path_env) {
        if let Some(path) = find_in_dir(&dir, &clean_cmd) {
            return Some(path);
        }
    }

    None
}

fn find_in_dir(dir: &Path, cmd: &str) -> Option<PathBuf> {
    for ext in EXECUTABLE_EXTENSIONS {
        let candidate = dir.join(format!("{}{}", cmd, ext));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let candidate = dir.join(cmd);
    candidate.exists().then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_extension_command_in_current_dir() {
        let dir =
            std::env::temp_dir().join(format!("winuxsh_lookup_current_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let command = dir.join("samplecmd.exe");
        fs::write(&command, "").unwrap();

        assert_eq!(find_command("samplecmd", &dir, None), Some(command));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_relative_path_against_shell_current_dir() {
        let dir =
            std::env::temp_dir().join(format!("winuxsh_lookup_relative_{}", std::process::id()));
        fs::create_dir_all(dir.join("bin")).unwrap();
        let command = dir.join("bin").join("tool");
        fs::write(&command, "").unwrap();

        assert_eq!(find_command("bin/tool", &dir, None), Some(command));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn searches_supplied_path_env() {
        let root = std::env::temp_dir().join(format!("winuxsh_lookup_path_{}", std::process::id()));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let command = bin.join("pathcmd.exe");
        fs::write(&command, "").unwrap();
        let path_env = std::env::join_paths([bin.as_path()]).unwrap();
        let path_env = path_env.to_string_lossy();

        assert_eq!(
            find_command("pathcmd", &root, Some(&path_env)),
            Some(command)
        );

        let _ = fs::remove_dir_all(root);
    }
}
