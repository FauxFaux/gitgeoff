use std::path::Path;

use failure::Error;
use failure::ResultExt;
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

pub fn variance_from_origin_head(repo: &git2::Repository) -> Result<usize, Error> {
    let local = repo.revparse_single("HEAD")?.id();
    let remote = repo.revparse_single("origin/REMOTE_HEAD")?.id();
    Ok(if local != remote {
        1
    } else {
        0
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

    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, _, _| {
        // TODO: do we need to parse this out of the URL, or have it as config?
        git2::Cred::ssh_key_from_agent("git")
    });

    // text from the remote, e.g. "counting objects"
    cb.sideband_progress(|message| {
        info!("{:?}: {:?}", dest, String::from_utf8_lossy(message));
        true
    });

    cb.transfer_progress(|progress| {
        info!(
            "{:?}: {:?}",
            dest,
            [
                progress.indexed_deltas(),
                progress.indexed_objects(),
                progress.total_objects(),
                progress.received_bytes(),
                progress.received_objects(),
                progress.local_objects(),
                progress.total_deltas(),
            ]
        );
        true
    });

    let mut options = git2::FetchOptions::default();
    options.remote_callbacks(cb);

    info!("fetching {:?} -> {:?}", url, dest);
    origin
        .fetch(&[], Some(&mut options), None)
        .with_context(|_| "fetching")?;

    Ok(())
}
