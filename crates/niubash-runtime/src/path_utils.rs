//! Shared Windows-native path helpers.
//!
//! Niubash accepts both native Windows paths (`C:/Users/me`) and compatibility
//! slash-drive paths (`/c/Users/me`). Host-facing code should normalize those
//! inputs before comparing or writing filesystem paths.

use std::path::PathBuf;

pub(crate) fn shell_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let names = ["USERPROFILE", "HOME"];
    #[cfg(not(windows))]
    let names = ["HOME", "USERPROFILE"];

    names
        .into_iter()
        .find_map(home_env_path)
        .or_else(dirs::home_dir)
        .map(normalize_existing_host_path)
}

pub(crate) fn home_env_path(name: &str) -> Option<PathBuf> {
    let value = std::env::var_os(name).filter(|value| !value.is_empty())?;
    Some(PathBuf::from(shell_path_to_host_path(
        value.to_string_lossy().as_ref(),
    )))
}

pub(crate) fn shell_path_to_host_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    if cfg!(windows) {
        let bytes = normalized.as_bytes();
        if bytes.len() == 2 && bytes[0] == b'/' && (bytes[1] as char).is_ascii_alphabetic() {
            let drive = (bytes[1] as char).to_ascii_uppercase();
            return format!("{drive}:/");
        }
        if bytes.len() >= 3
            && bytes[0] == b'/'
            && (bytes[1] as char).is_ascii_alphabetic()
            && bytes[2] == b'/'
        {
            let drive = (bytes[1] as char).to_ascii_uppercase();
            return format!("{drive}:{}", &normalized[2..]);
        }
    }
    value.to_string()
}

/// Backing directory for the shell-visible `/tmp` on the host filesystem.
/// Mirrors rubash's Windows resolution for host-side readers that do not own
/// the executor (completion, syntax highlighting): process TMPDIR, then TEMP,
/// then TMP; a value that is empty or spells the virtual `/tmp` itself falls
/// back to the OS temp dir (unixwin/niubash#94).
pub(crate) fn host_tmp_dir() -> PathBuf {
    for name in ["TMPDIR", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(name) {
            let normalized = value.replace('\\', "/");
            let normalized = normalized.trim_end_matches('/');
            if !normalized.is_empty() && normalized != "/tmp" && !normalized.starts_with("/tmp/") {
                return PathBuf::from(value);
            }
        }
    }
    std::env::temp_dir()
}

/// Backing directory for the shell-visible `/var/tmp`. Kept under a `var/tmp`
/// child of the same temp source rubash uses (`safe_temp_dir_string()`:
/// process TMPDIR, TEMP, TMP, then the current-dir `target` fallback) so it
/// cannot collide with `/tmp` or with unrelated %TEMP% contents
/// (unixwin/niubash#94).
pub(crate) fn host_var_tmp_dir() -> PathBuf {
    for name in ["TMPDIR", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(name) {
            if !value.is_empty() && !value.contains('\0') {
                return PathBuf::from(value).join("var").join("tmp");
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("var")
        .join("tmp")
}

pub(crate) fn normalize_existing_host_path(path: PathBuf) -> PathBuf {
    let Ok(canonical) = std::fs::canonicalize(&path) else {
        return path;
    };
    #[cfg(windows)]
    {
        normalize_windows_extended_path(canonical).unwrap_or(path)
    }
    #[cfg(not(windows))]
    {
        canonical
    }
}

#[cfg(windows)]
fn normalize_windows_extended_path(path: PathBuf) -> Option<PathBuf> {
    let path = path.to_string_lossy();
    let without_prefix = path
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| path.strip_prefix(r"\\?\").map(str::to_string))
        .unwrap_or_else(|| path.to_string());
    Some(PathBuf::from(without_prefix))
}
