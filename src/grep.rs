use std::path::Path;

use failure::Error;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use super::config;
use config::Spec;

pub(crate) fn grep(pattern: &str) -> Result<(), Error> {
    config::load()?
        .into_par_iter()
        .map(|s: Spec| -> Result<(), Error> {
            let dest = Path::new(s.local_dir()?);
            if !dest.exists() {
                return Ok(());
            }
            let repo = git2::Repository::open(dest)?;
            let tree_obj = repo.revparse_single("origin/REMOTE_HEAD")?.peel_to_tree()?;
            tree_obj.walk(git2::TreeWalkMode::PostOrder, |dir, entry| {
                match entry.kind() {
                    Some(git2::ObjectType::Blob) => (),
                    _ => return git2::TreeWalkResult::Ok,
                };
                let object = entry.to_object(&repo).expect("TODO: can't load blob");

                let content = object.as_blob().expect("type checked above").content();

                if twoway::find_bytes(content, pattern.as_bytes()).is_some() {
                    println!("{:?} {:?} {:?}", dest, dir, entry.name());
                }

                git2::TreeWalkResult::Ok
            })?;
            Ok((()))
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}
