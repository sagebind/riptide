//! Helper functions for working with command search paths.

use auto_enums::*;
use libc::{access, X_OK};
use std::env::split_paths;
use std::ffi::CString;
use std::iter;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};

/// Determine whether the given path is a file that can be executed.
pub fn is_executable(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();

    let metadata = match path.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };

    if !metadata.is_file() {
        return false;
    }

    if metadata.mode() & 0o111 == 0 {
        return false;
    }

    unsafe {
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        if access(path.as_ptr(), X_OK) != 0 {
            return false;
        }
    }

    true
}

/// Find an executable command using the given search path.
pub fn find_executable(command: &str, search_path: &str) -> Option<PathBuf> {
    find_all_executable(command, search_path).next()
}

/// Find all matches for an executable command using the given search path.
pub fn find_all_executable<'a: 'r, 'b: 'r, 'r>(command: &'a str, search_path: &'b str) -> impl Iterator<Item=PathBuf> + 'r {
    find_all_executable_in_paths(command, split_paths(search_path))
}

/// Find all matches for an executable command in the given paths.
fn find_all_executable_in_paths<'a: 'r, 'b: 'r, 'r, P: AsRef<Path> + 'r>(command: &'a str, paths: impl IntoIterator<Item=P> + 'b) -> impl Iterator<Item=PathBuf> + 'r {
    find_all_in_paths(command, paths).filter(move |path| is_executable(path))
}

/// Find all matches for a file in the given paths.
#[auto_enum(Iterator)]
fn find_all_in_paths<'a: 'r, 'b: 'r, 'r, P: AsRef<Path>>(command: &'a str, paths: impl IntoIterator<Item=P> + 'b) -> impl Iterator<Item=PathBuf> + 'r {
    if command.contains('/') {
        iter::once(PathBuf::from(command))
    } else {
        paths.into_iter().map(move |path| path.as_ref().join(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_executable() {
        assert_eq!(find_executable("sh", "/bin:/usr/bin"), Some("/bin/sh".into()));
    }

    #[test]
    fn test_find_all_executable() {
        assert_eq!(find_all_executable("sh", "/bin:/usr/bin").next(), Some("/bin/sh".into()));
    }

    #[test]
    fn test_find_all_executable_in_paths() {
        assert_eq!(find_all_executable_in_paths("sh", &["/bin"]).next(), Some("/bin/sh".into()));
    }
}
