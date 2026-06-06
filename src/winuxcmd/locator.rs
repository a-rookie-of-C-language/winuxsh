use std::path::{Path, PathBuf};

use crate::command_lookup;

const WINUXCMD_CANDIDATES: &[&str] = &["winuxcmd.exe", "coreutils.exe", "uutils.exe"];

pub fn find_in_path(current_dir: &Path, path_env: Option<&str>) -> Option<PathBuf> {
    WINUXCMD_CANDIDATES
        .iter()
        .find_map(|name| command_lookup::find_command(name, current_dir, path_env))
}

pub fn find_bundled(current_dir: &Path) -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate = exe_dir.join("winuxcmd").join("winuxcmd.exe");
            if candidate.exists() {
                return Some(candidate);
            }

            if let Some(target_dir) = exe_dir.parent() {
                if let Some(repo_root) = target_dir.parent() {
                    let candidate = repo_root
                        .join("utils")
                        .join("winuxcmd")
                        .join("winuxcmd.exe");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    let repo_candidate = current_dir
        .join("utils")
        .join("winuxcmd")
        .join("winuxcmd.exe");
    repo_candidate.exists().then_some(repo_candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_candidate_from_supplied_path() {
        let root =
            std::env::temp_dir().join(format!("winuxsh_winuxcmd_locator_{}", std::process::id()));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let command = bin.join("uutils.exe");
        fs::write(&command, "").unwrap();
        let path_env = std::env::join_paths([bin.as_path()]).unwrap();
        let path_env = path_env.to_string_lossy();

        assert_eq!(find_in_path(&root, Some(&path_env)), Some(command));

        let _ = fs::remove_dir_all(root);
    }
}
