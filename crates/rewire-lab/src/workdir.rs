//! The per-run work directory, and removing it reliably.
//!
//! Learner code can `chmod 000` directories under `/work`, which makes a plain
//! `remove_dir_all` fail and leaves the tree behind in the host temp dir. So
//! before removing, we walk the tree and give every directory back u+rwx.
//! The walk never follows a symlink: each directory is opened relative to its
//! parent's fd with `O_PATH | O_NOFOLLOW | O_DIRECTORY`, its mode is changed
//! through that fd's `/proc/self/fd/N` link (which names the opened inode
//! itself, not a path), and it is then reopened for reading through the same
//! link. A symlink fails the `O_NOFOLLOW | O_DIRECTORY` open and is skipped;
//! `remove_dir_all` (symlink-safe in std) then unlinks it like a file.

use std::ffi::CString;
use std::fs::{self, File, OpenOptions, ReadDir};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// A fresh `rewire-lab-*` temp dir that is removed on drop, even if learner
/// code locked parts of it.
pub(crate) struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    pub(crate) fn new() -> io::Result<Self> {
        let dir = tempfile::Builder::new().prefix("rewire-lab-").tempdir()?;
        Ok(Self { path: dir.keep() })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        // Never panic in drop: log and retry once.
        for attempt in 1..=2 {
            match remove_tree(&self.path) {
                Ok(()) => return,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return,
                Err(e) => eprintln!(
                    "rewire-lab: could not remove work dir {} (attempt {attempt}): {e}",
                    self.path.display()
                ),
            }
        }
    }
}

/// Restores u+rwx on every directory (without following symlinks), then
/// removes the tree.
pub(crate) fn remove_tree(root: &Path) -> io::Result<()> {
    // Best effort: whatever this can't unlock, remove_dir_all reports.
    let _ = unlock_tree(root);
    fs::remove_dir_all(root)
}

fn unlock_tree(root: &Path) -> io::Result<()> {
    let root_c = CString::new(root.as_os_str().as_bytes())?;
    let root_dir = open_dir_unlocked(libc::AT_FDCWD, &root_c)?;
    let entries = read_dir_of(&root_dir)?;
    // One open directory (plus its listing) per level of depth, not per
    // directory, so wide trees don't use up fds.
    let mut stack: Vec<(File, ReadDir)> = vec![(root_dir, entries)];
    while let Some(top) = stack.len().checked_sub(1) {
        let Some(entry) = stack[top].1.next() else {
            stack.pop();
            continue;
        };
        let Ok(entry) = entry else { continue };
        // d_type based; does not follow symlinks.
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let Ok(name) = CString::new(entry.file_name().as_bytes()) else {
            continue;
        };
        let parent = stack[top].0.as_raw_fd();
        // Deeper than the fd limit allows: leave the rest to remove_dir_all.
        let Ok(child) = open_dir_unlocked(parent, &name) else {
            continue;
        };
        let Ok(child_entries) = read_dir_of(&child) else {
            continue;
        };
        stack.push((child, child_entries));
    }
    Ok(())
}

/// Opens `name` (relative to `parent`) as a directory without following a
/// symlink, adds u+rwx to its mode, and returns it opened for reading.
fn open_dir_unlocked(parent: RawFd, name: &CString) -> io::Result<File> {
    // SAFETY: valid NUL-terminated path; the fd is owned right after.
    let raw = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_PATH | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is a fresh fd we own.
    let handle = unsafe { OwnedFd::from_raw_fd(raw) };
    let link = fd_link(&handle);
    // The magic link resolves to the inode `handle` refers to, never to
    // whatever a path might point at now.
    let mode = fs::metadata(&link)?.permissions().mode();
    if mode & 0o700 != 0o700 {
        fs::set_permissions(&link, fs::Permissions::from_mode((mode | 0o700) & 0o7777))?;
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(&link)
}

fn read_dir_of(dir: &File) -> io::Result<ReadDir> {
    fs::read_dir(fd_link(dir))
}

fn fd_link(fd: &impl AsRawFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn removes_locked_nested_dirs_without_touching_symlink_targets() {
        let outside = tempfile::tempdir().unwrap();
        let target_dir = outside.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o500)).unwrap();

        let w = WorkDir::new().unwrap();
        let root = w.path().to_path_buf();
        fs::create_dir_all(root.join("a/b/c")).unwrap();
        fs::write(root.join("a/b/c/f"), "x").unwrap();
        symlink(&target_dir, root.join("a/link")).unwrap();
        fs::set_permissions(root.join("a/b"), fs::Permissions::from_mode(0o0)).unwrap();
        fs::set_permissions(root.join("a"), fs::Permissions::from_mode(0o0)).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o0)).unwrap();

        drop(w);
        assert!(root.symlink_metadata().is_err(), "work dir removed");
        let mode = fs::metadata(&target_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o500, "symlink target's mode untouched");
        assert!(target_dir.is_dir(), "symlink target not removed");
        fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o700)).unwrap();
    }
}
