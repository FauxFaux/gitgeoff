use std::str::FromStr;

use failure::format_err;
use failure::Error;

#[derive(Clone, Debug)]
pub enum GitUrl {
    Real(url::Url),
    // TODO: maybe this should just be.. other?
    Ssh(String),
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
                .ok_or_else(|| format_err!("no path in {:?}", url))?
                .last()
                .ok_or_else(|| format_err!("empty path in {:?}", url))?,
            GitUrl::Ssh(ssh) => {
                // git(1) parses `git:foo@example.com:1337:foo` as `git` being the hostname
                let path = match ssh.find(':') {
                    Some(pos) => &ssh[pos..],
                    None => ssh,
                };

                path.split('/')
                    .last()
                    .ok_or_else(|| format_err!("empty path in {:?}", ssh))?
            }
        };

        Ok(if base_name.ends_with(".git") {
            &base_name[..base_name.len() - 4]
        } else {
            base_name
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use failure::Error;

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
    #[ignore]
    fn broken_cases() -> Result<(), Error> {
        // parse ignoring the colon (as a path), then can't treat as a path, I think
        GitUrl::from_str("gh:FauxFaux/gitgeoff.git")?;
        Ok(())
    }
}
