use anyhow::Result;

pub fn infect() -> Result<()> {
    let repo = git2::Repository::open(".")?;
    add_if_missing(&repo)?;
    Ok(())
}

fn add_if_missing(repo: &git2::Repository) -> Result<()> {
    if !fetches_remote_head(repo)? {
        repo.remote_add_fetch("origin", "+HEAD:refs/remotes/origin/REMOTE_HEAD")?;
    }
    Ok(())
}

pub fn fetches_remote_head(repo: &git2::Repository) -> Result<bool> {
    let config = repo.config()?;
    let mut entries = config.entries(Some("remote.origin.fetch"))?;
    while let Some(entry) = entries.next() {
        if entry?.value().unwrap_or("") == "+HEAD:refs/remotes/origin/REMOTE_HEAD" {
            return Ok(true);
        }
    }

    Ok(false)
}
