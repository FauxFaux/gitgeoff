use std::path::Path;

use failure::Error;
use failure::ResultExt;
use log::info;

pub fn fetch_to(url: &str, dest: &Path) -> Result<(), Error> {
    match git2::Repository::open_bare(&dest) {
        Ok(_) => return Ok(()),
        Err(ref e) if e.code() == git2::ErrorCode::NotFound => (),
        Err(e) => Err(e)?,
    };

    let repo = git2::Repository::init_bare(&dest)?;

    let mut origin = repo.remote("origin", url)?;
    repo.config()?.remove("remote.origin.fetch")?;
    repo.remote_add_fetch("origin", "+refs/heads/*:refs/heads/*")?;

    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, _, _| {
        // TODO: do we need to parse this out of the URL, or have it as config?
        git2::Cred::ssh_key_from_agent("git")
    });
    let mut options = git2::FetchOptions::default();
    options.remote_callbacks(cb);

    info!("fetching {:?} -> {:?}", url, dest);
    origin
        .fetch(&[], Some(&mut options), None)
        .with_context(|_| "fetching")?;

    Ok(())
}
