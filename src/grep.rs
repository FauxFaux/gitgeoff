use std::path::Path;

use failure::format_err;
use failure::Error;
use failure::ResultExt;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::Lossy;
use grep_searcher::Searcher;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use super::config;
use config::Spec;

pub(crate) fn grep(pattern: &str) -> Result<(), Error> {
    config::load()?
        .into_par_iter()
        .map(|s: Spec| -> Result<(), Error> {
            let dest = s.local_dir()?;
            if !Path::new(dest).exists() {
                return Ok(());
            }
            let repo = git2::Repository::open(dest)?;
            grep_in(pattern, dest, s.html_url()?, &repo)?;
            Ok(())
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

fn grep_in(
    pattern: &str,
    prefix: &str,
    html_url: &str,
    repo: &git2::Repository,
) -> Result<(), Error> {
    let matcher = RegexMatcher::new(pattern)?;
    let tree_obj = repo
        .revparse_single("origin/REMOTE_HEAD")
        .with_context(|_| format_err!("looking in {:?}", prefix))?
        .peel_to_tree()?;
    let mut err = Vec::new();

    tree_obj.walk(git2::TreeWalkMode::PostOrder, |dir, entry| {
        match entry.kind() {
            Some(git2::ObjectType::Blob) => (),
            _ => return git2::TreeWalkResult::Ok,
        };
        let object = entry.to_object(&repo).expect("TODO: can't load blob");

        let content = object.as_blob().expect("type checked above").content();

        let path = format!("{}{}", dir, entry.name().expect("blobs have names"));

        let status = Searcher::new().search_slice(
            &matcher,
            content,
            Lossy(|lnum, line| {
                println!(
                    "{}/{} {}: {}",
                    prefix,
                    href(&path, &format!("{}/blob/HEAD/{}#L{}", html_url, path, lnum)),
                    lnum,
                    line.trim_end()
                );
                Ok(true)
            }),
        );

        match status {
            Ok(()) => (),
            Err(e) => err.push(e),
        }

        git2::TreeWalkResult::Ok
    })?;

    // TODO: ..and the other errors, if any?
    if let Some(e) = err.into_iter().next() {
        Err(e)?;
    }

    Ok(())
}

fn href(label: &str, url: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, label)
}
