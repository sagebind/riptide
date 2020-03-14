use directories::ProjectDirs;
use std::{fs, io, path::{Path, PathBuf}};

lazy_static::lazy_static! {
    static ref PROJECT_DIRS: Option<ProjectDirs> = directories::ProjectDirs::from("sh.riptide", "", "Riptide");
}

pub fn config_dir() -> io::Result<&'static Path> {
    PROJECT_DIRS.as_ref()
        .unwrap()
        .config_dir()
        .mkdirs()
}

pub fn data_dir() -> io::Result<&'static Path> {
    PROJECT_DIRS.as_ref()
        .unwrap()
        .data_dir()
        .mkdirs()
}

pub fn config_file() -> io::Result<PathBuf> {
    config_dir().map(|dir| dir.join("riptide.toml"))
}

pub fn history_db() -> io::Result<PathBuf> {
    data_dir().map(|dir| dir.join("history.db"))
}

trait Mkdirs: AsRef<Path> + Sized {
    fn mkdirs(self) -> io::Result<Self> {
        fs::create_dir_all(self.as_ref())?;
        Ok(self)
    }
}

impl<P: AsRef<Path>> Mkdirs for P {}
