//! Command-file tailing: seek-offset reads of newly appended lines, with
//! truncation reset, matching swiftDialog's watcher semantics. Pre-existing
//! content is skipped unless we created the file ourselves.
//!
//! Default paths: `/var/tmp/dialog.log` on macOS (upstream),
//! `%PUBLIC%\dialog.log` on Windows (design D4 — writable by Users and
//! SYSTEM out of the box, preserving the cross-privilege contract).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub fn default_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let public = std::env::var("PUBLIC").unwrap_or_else(|_| r"C:\Users\Public".to_string());
        PathBuf::from(public).join("dialog.log")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/var/tmp/dialog.log")
    }
}

pub struct CommandFileTail {
    path: PathBuf,
    offset: u64,
    /// Carry for a final line not yet terminated by \n.
    partial: String,
}

impl CommandFileTail {
    /// Open (creating world-writable if absent) and position at the end of
    /// existing content so only new appends are processed.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let created = !path.exists();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)?;

        if created {
            make_world_writable(path, &file)?;
        }

        let offset = if created { 0 } else { file.metadata()?.len() };
        Ok(CommandFileTail {
            path: path.to_path_buf(),
            offset,
            partial: String::new(),
        })
    }

    /// Read newly appended complete lines. Resets to the start when the
    /// file was truncated. Missing file yields no lines (it may be
    /// recreated by a writer later).
    pub fn poll(&mut self) -> Vec<String> {
        let Ok(mut file) = File::open(&self.path) else {
            return Vec::new();
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            // truncated — start over (command-file-ipc spec)
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset {
            return Vec::new();
        }
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return Vec::new();
        }
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            return Vec::new();
        }
        self.offset = len;

        let mut text = std::mem::take(&mut self.partial);
        text.push_str(&buf);
        let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        // Last element is either "" (text ended with \n) or an incomplete
        // line to carry into the next poll.
        if let Some(last) = lines.pop() {
            if !last.is_empty() {
                self.partial = last;
            }
        }
        lines.retain(|l| !l.trim().is_empty());
        lines
    }
}

#[cfg(unix)]
fn make_world_writable(path: &Path, _file: &File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))
}

#[cfg(not(unix))]
fn make_world_writable(path: &Path, _file: &File) -> std::io::Result<()> {
    // Everyone-modify ACL so SYSTEM services and other users can write
    // (mirrors upstream's 0666). icacls ships with Windows.
    let _ = std::process::Command::new("icacls")
        .arg(path)
        .args(["/grant", "Everyone:M"])
        .output();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("dialog-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn reads_only_new_appends() {
        let path = temp("appends");
        std::fs::write(&path, "old: content\n").unwrap();
        let mut tail = CommandFileTail::open(&path).unwrap();
        assert!(tail.poll().is_empty());

        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "title: New").unwrap();
        writeln!(f, "progress: 3").unwrap();
        assert_eq!(tail.poll(), vec!["title: New", "progress: 3"]);
        assert!(tail.poll().is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn truncation_resets_to_start() {
        // spec scenario: file truncated to zero length, then a new command
        // appended — the new command is processed normally. (A one-shot
        // rewrite to LONGER content is undetectable by offset arithmetic,
        // same as upstream's file-handle reader.)
        let path = temp("trunc");
        std::fs::write(&path, "a: 1\nb: 2\n").unwrap();
        let mut tail = CommandFileTail::open(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        assert!(tail.poll().is_empty()); // poller observes the empty file
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "title: fresh").unwrap();
        assert_eq!(tail.poll(), vec!["title: fresh"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn partial_lines_carry_over() {
        let path = temp("partial");
        let mut tail = CommandFileTail::open(&path).unwrap();
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        write!(f, "title: ha").unwrap();
        assert!(tail.poll().is_empty());
        writeln!(f, "lf then whole").unwrap();
        assert_eq!(tail.poll(), vec!["title: half then whole"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn created_file_is_world_writable_on_unix() {
        let path = temp("perms");
        let _tail = CommandFileTail::open(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o666);
        }
        let _ = std::fs::remove_file(&path);
    }
}
