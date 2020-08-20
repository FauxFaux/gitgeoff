use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Error;
use lazy_static::lazy_static;

#[derive(Clone, Debug)]
pub enum GitUrl {
    Real(url::Url),
    // TODO: maybe this should just be.. other?
    Ssh(String),
}

pub enum Provider {
    GithubCom { org: String, repo: String },
}

lazy_static! {
    static ref GITHUB_SSH: regex::Regex =
        regex::Regex::new(r"git@[^:/]*github.com:/?([^/]+)/([^/]+)").expect("static regex");
}

impl FromStr for GitUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<GitUrl, Error> {
        match url::Url::from_str(s) {
            Ok(url) => Ok(GitUrl::Real(url)),
            Err(url_error) => {
                if s.contains(':') {
                    Ok(GitUrl::Ssh(s.to_string()))
                } else {
                    Err(url_error)?
                }
            }
        }
    }
}

impl GitUrl {
    pub fn as_str(&self) -> &str {
        match self {
            GitUrl::Real(url) => url.as_str(),
            GitUrl::Ssh(string) => string.as_str(),
        }
    }

    pub fn local_dir(&self) -> Result<&str, Error> {
        let base_name = match self {
            GitUrl::Real(url) => url
                .path_segments()
                .ok_or_else(|| anyhow!("no path in {:?}", url))?
                .last()
                .ok_or_else(|| anyhow!("empty path in {:?}", url))?,
            GitUrl::Ssh(url) => strip_to_colon(&url)
                .split('/')
                .last()
                .ok_or_else(|| anyhow!("empty path in {:?}", url))?,
        };

        Ok(strip_git(&base_name))
    }

    pub fn provider(&self) -> Option<Provider> {
        Some(match self {
            GitUrl::Real(url) => {
                if !url.host_str()?.ends_with("github.com") {
                    return None;
                }
                let mut segments = url.path_segments()?;
                Provider::GithubCom {
                    org: segments.next()?.to_string(),
                    repo: strip_git(segments.next()?).to_string(),
                }
            }
            GitUrl::Ssh(url) => {
                let matches = GITHUB_SSH.captures(url)?;
                Provider::GithubCom {
                    org: matches.get(1)?.as_str().to_string(),
                    repo: strip_git(matches.get(2)?.as_str()).to_string(),
                }
            }
        })
    }
}

impl Provider {
    pub fn html_browse_path(&self, branch: Option<&str>, path: &str, line: Option<u64>) -> String {
        match self {
            Provider::GithubCom { org, repo } => format!(
                "https://github.com/{org}/{repo}/blob/{branch}/{path}{line}",
                org = org,
                repo = repo,
                branch = branch.unwrap_or("HEAD"),
                path = path,
                line = line.map(|n| format!("#L{}", n)).unwrap_or_else(String::new)
            ),
        }
    }
}

/// git(1) parses `git:foo@example.com:1337:foo` as `git` being the hostname
fn strip_to_colon(ssh: &str) -> &str {
    match ssh.find(':') {
        Some(pos) => &ssh[pos..],
        None => ssh,
    }
}

fn strip_git(base_name: &str) -> &str {
    if base_name.ends_with(".git") {
        &base_name[..base_name.len() - 4]
    } else {
        base_name
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::Error;

    use super::GitUrl;

    #[test]
    fn get_local_dir() -> Result<(), Error> {
        assert_eq!(
            "gitgeoff",
            GitUrl::from_str("https://github.com/FauxFaux/gitgeoff")?.local_dir()?
        );
        assert_eq!(
            "gitgeoff",
            GitUrl::from_str("https://github.com/FauxFaux/gitgeoff.git")?.local_dir()?
        );
        assert_eq!(
            "gitgeoff",
            GitUrl::from_str("git@github.com:FauxFaux/gitgeoff")?.local_dir()?
        );
        assert_eq!(
            "gitgeoff",
            GitUrl::from_str("git@github.com:FauxFaux/gitgeoff.git")?.local_dir()?
        );
        assert_eq!(
            "gitgeoff",
            GitUrl::from_str("git@github.com:/FauxFaux/gitgeoff.git")?.local_dir()?
        );
        Ok(())
    }

    #[test]
    fn get_provider() -> Result<(), Error> {
        assert_eq!(
            "https://github.com/FauxFaux/gitgeoff/blob/HEAD/foo/bar.txt#L7",
            GitUrl::from_str("git@github.com:/FauxFaux/gitgeoff.git")?
                .provider()
                .unwrap()
                .html_browse_path(None, "foo/bar.txt", Some(7))
        );
        Ok(())
    }

    #[test]
    #[ignore]
    fn broken_cases() -> Result<(), Error> {
        // parse ignoring the colon (as a path), then can't treat as a path, I think
        GitUrl::from_str("gh:FauxFaux/gitgeoff.git")?;
        Ok(())
    }
}
