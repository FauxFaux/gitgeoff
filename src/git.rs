use std::path::Path;

use failure::Error;
use failure::ResultExt;
use git2::Oid;
use git2::Remote;
use git2::Repository;
use git2::Status;
use log::info;

fn if_found<T>(res: Result<T, git2::Error>) -> Result<Option<T>, Error> {
    match res {
        Ok(t) => Ok(Some(t)),
        Err(ref e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e)?,
    }
}

pub fn first_statuses(repo: &git2::Repository) -> Result<Vec<String>, Error> {
    let mut dirty = false;
    let statuses = repo.statuses(None)?;
    Ok(statuses
        .iter()
        .filter(|status| !status.status().is_ignored())
        .take(3)
        .map(|status| {
            format!(
                "{} {:?}",
                match status.status() {
                    Status::INDEX_NEW => "add".to_string(),
                    Status::WT_NEW => "new".to_string(),
                    Status::INDEX_MODIFIED | Status::WT_MODIFIED => "mod".to_string(),
                    Status::INDEX_DELETED | Status::WT_DELETED => "del".to_string(),
                    Status::INDEX_RENAMED | Status::WT_RENAMED => "mov".to_string(),
                    Status::CONFLICTED => "CON".to_string(),
                    other => format!("?{:?}?", other),
                },
                status.path().unwrap_or("?")
            )
        })
        .collect())
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Variance {
    Equal,
    NotOnBranch,

    // directly ahead of the remote
    Ahead(usize),

    // directly behind the remote (fast forward)
    Behind(usize),

    Diverged { local: usize, remote: usize },
}

pub fn variance_from_origin_head(repo: &git2::Repository) -> Result<Variance, Error> {
    let head = repo.head()?;
    if !head.is_branch() {
        return Ok(Variance::NotOnBranch);
    }

    let local = head.peel_to_commit()?.id();
    let remote = repo.revparse_single("origin/REMOTE_HEAD")?.id();

    if local == remote {
        return Ok(Variance::Equal);
    }

    let base = repo.merge_base(local, remote)?;

    let ahead = commits_in(&repo, local, base)?;
    let behind = commits_in(&repo, remote, base)?;

    Ok(if base == remote && 0 != ahead {
        Variance::Ahead(ahead)
    } else if base == local && 0 != behind {
        Variance::Behind(behind)
    } else {
        Variance::Diverged {
            local: ahead,
            remote: behind,
        }
    })
}

pub fn clone_or_fetch(url: &str, dest: &Path) -> Result<(), Error> {
    let repo = match if_found(git2::Repository::open_bare(&dest))? {
        Some(repository) => repository,
        None => git2::Repository::init_bare(&dest)?,
    };

    let mut origin = match if_found(repo.find_remote("origin"))? {
        Some(origin) => origin,
        None => repo.remote("origin", url)?,
    };

    if_found(repo.config()?.remove_multivar("remote.origin.fetch", ".*"))?;
    repo.remote_add_fetch("origin", "+refs/heads/*:refs/heads/*")?;

    info!("fetching {:?} -> {:?}", url, dest);
    do_fetch(&repo, &mut origin, |p| {
        info!("{:?}: {:?}", dest, p);
    })?;

    Ok(())
}

pub fn fetch_origin_default(repo: &Repository) -> Result<(), Error> {
    let mut origin = repo.find_remote("origin")?;
    do_fetch(&repo, &mut origin, |p| {
        info!("{:?}", p);
    })?;
    Ok(())
}

#[derive(Debug)]
enum Progress {
    Sideband(String),
    Transfer([usize; 7]),
}

fn do_fetch<F: Fn(Progress)>(
    repo: &Repository,
    origin: &mut Remote,
    progress: F,
) -> Result<(), Error> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, _, _| {
        // TODO: do we need to parse this out of the URL, or have it as config?
        git2::Cred::ssh_key_from_agent("git")
    });

    // text from the remote, e.g. "counting objects"
    cb.sideband_progress(|message| {
        progress(Progress::Sideband(
            String::from_utf8_lossy(message).to_string(),
        ));
        true
    });

    cb.transfer_progress(|counts| {
        progress(Progress::Transfer([
            counts.local_objects(),
            counts.received_objects(),
            counts.indexed_objects(),
            counts.total_objects(),
            counts.indexed_deltas(),
            counts.total_deltas(),
            counts.received_bytes(),
        ]));
        true
    });

    let mut options = git2::FetchOptions::default();
    options.remote_callbacks(cb);

    origin
        .fetch(&[], Some(&mut options), None)
        .with_context(|_| "fetching")?;

    Ok(())
}

pub fn commits_in(repo: &Repository, start: Oid, end: Oid) -> Result<usize, Error> {
    let mut walk = repo.revwalk()?;
    walk.push(start)?;
    walk.hide(end)?;
    Ok(walk.count())
}

#[cfg(test)]
mod tests {
    #[test]
    fn revwalk_direction() -> Result<(), failure::Error> {
        let repo = git2::Repository::open(".")?;
        let two_back = repo.revparse_single("HEAD~2")?;

        let head = repo.revparse_single("HEAD")?.id();

        let mut walk = repo.revwalk()?;
        walk.push_head()?;
        walk.hide(two_back.id())?;
        assert_eq!(2, walk.count());
        assert_eq!(2, super::commits_in(&repo, head, two_back.id())?);

        let mut walk = repo.revwalk()?;
        walk.hide_head()?;
        walk.push(two_back.id())?;
        assert_eq!(0, walk.count());

        Ok(())
    }
}
