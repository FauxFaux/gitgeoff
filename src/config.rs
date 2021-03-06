use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use url::Url;

use crate::git_url::GitUrl;

#[derive(Clone)]
pub struct Spec {
    pub url: GitUrl,
    pub tags: HashSet<String>,
}

impl Spec {
    pub fn html_url(&self) -> Result<&str, Error> {
        Ok(self.url.as_str())
    }
}

pub fn load() -> Result<Vec<Spec>, Error> {
    let mut ret = Vec::with_capacity(20);
    let file = io::BufReader::new(fs::File::open(".gitgeoff")?);
    for line in file.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split(|c: char| c.is_whitespace());
        let line = parts.next().ok_or_else(|| anyhow!("invalid config line"))?;

        let url =
            GitUrl::from_str(line).with_context(|| anyhow!("parsing config line {:?}", line))?;
        let mut tags = HashSet::with_capacity(4);
        for tag in parts {
            tags.insert(tag.to_string());
        }
        ret.push(Spec { url, tags });
    }

    Ok(ret)
}
