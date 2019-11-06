use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::str::FromStr;

use failure::err_msg;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use url::Url;

#[derive(Clone)]
pub struct Spec {
    pub url: Url,
    pub tags: HashSet<String>,
}

impl Spec {
    pub fn local_dir(&self) -> Result<&str, Error> {
        Ok(self
            .url
            .path_segments()
            .ok_or_else(|| format_err!("no path in {:?}", self.url))?
            .last()
            .ok_or_else(|| format_err!("empty path in {:?}", self.url))?)
    }

    pub fn html_url(&self) -> Result<&str, Error> {
        Ok(self.url.as_str())
    }
}

pub fn load() -> Result<Vec<Spec>, Error> {
    let mut ret = Vec::with_capacity(20);
    let file = io::BufReader::new(fs::File::open(".gitoff")?);
    for line in file.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split(|c: char| c.is_whitespace());
        let line = parts.next().ok_or_else(|| err_msg("invalid config line"))?;

        // TODO: LOL
        if line.starts_with("git@github.com:") {
            continue;
        }

        let url =
            Url::from_str(line).with_context(|_| format_err!("parsing config line {:?}", line))?;
        let mut tags = HashSet::with_capacity(4);
        for tag in parts {
            tags.insert(tag.to_string());
        }
        ret.push(Spec { url, tags });
    }

    Ok(ret)
}
