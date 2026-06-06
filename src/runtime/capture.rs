use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct TempCapture {
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl TempCapture {
    pub fn new(prefix: &str) -> Self {
        let id = NEXT_CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), id));

        Self {
            stdout_path: base.with_extension("stdout.tmp"),
            stderr_path: base.with_extension("stderr.tmp"),
        }
    }

    pub fn stdout_path(&self) -> &Path {
        &self.stdout_path
    }

    pub fn stderr_path(&self) -> &Path {
        &self.stderr_path
    }
}

impl Drop for TempCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.stdout_path);
        let _ = std::fs::remove_file(&self.stderr_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_removes_capture_files() {
        let stdout_path;
        let stderr_path;
        {
            let capture = TempCapture::new("winuxsh_capture_test");
            stdout_path = capture.stdout_path().to_path_buf();
            stderr_path = capture.stderr_path().to_path_buf();
            std::fs::write(&stdout_path, "out").unwrap();
            std::fs::write(&stderr_path, "err").unwrap();
            assert!(stdout_path.exists());
            assert!(stderr_path.exists());
        }

        assert!(!stdout_path.exists());
        assert!(!stderr_path.exists());
    }
}
