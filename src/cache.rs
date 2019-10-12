use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use failure::err_msg;
use failure::Error;

pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Cache, Error> {
        Ok(Cache { root: pick()? })
    }

    pub fn meta_github_org(&self, org: &str) -> Result<PathBuf, Error> {
        mkdirs(self.root.join("github").join(fs_safe_component(org)))
    }
}

fn mkdirs<P: AsRef<Path>>(path: P) -> Result<P, Error> {
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn pick() -> Result<PathBuf, Error> {
    let dir = from_env()?;

    fs::create_dir_all(&dir)?;

    Ok(dir)
}

fn from_env() -> Result<PathBuf, Error> {
    if let Some(dirs) = directories::ProjectDirs::from("xxx", "fau", clap::crate_name!()) {
        return Ok(dirs.cache_dir().to_path_buf());
    };

    if let Some(dirs) = directories::UserDirs::new() {
        let mut home = dirs.home_dir().to_path_buf();
        home.push(".cache");
        home.push(clap::crate_name!());
        return Ok(home);
    }

    Err(err_msg("no HOME so couldn't find a cache dir"))
}

fn fs_safe_component(path: &str) -> String {
    path.replace(|c: char| !c.is_alphanumeric(), "_")
}
