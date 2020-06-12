use directories::ProjectDirs;
use std::{fs, io, path::{Path, PathBuf}};

pub fn config_dir() -> io::Result<PathBuf> {
    project_dirs()
        .config_dir()
        .to_path_buf()
        .mkdirs()
}

pub fn data_dir() -> io::Result<PathBuf> {
    project_dirs()
        .data_dir()
        .to_path_buf()
        .mkdirs()
}

pub fn config_file() -> io::Result<PathBuf> {
    config_dir().map(|dir| dir.join("riptide.toml"))
}

pub fn history_db() -> io::Result<PathBuf> {
    data_dir().map(|dir| dir.join("history.db"))
}

fn project_dirs() -> ProjectDirs {
    directories::ProjectDirs::from("sh.riptide", "", "Riptide").unwrap()
}

trait Mkdirs: AsRef<Path> + Sized {
    fn mkdirs(self) -> io::Result<Self> {
        fs::create_dir_all(self.as_ref())?;
        Ok(self)
    }
}

impl<P: AsRef<Path>> Mkdirs for P {}
