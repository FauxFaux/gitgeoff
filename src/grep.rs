use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::Lossy;
use grep_searcher::Searcher;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use super::config;
use crate::git_url::Provider;
use config::Spec;

pub fn grep(pattern: &str, globs: &[&str]) -> Result<(), Error> {
    let globs = {
        let mut builder = globset::GlobSetBuilder::new();
        for &glob in globs {
            builder.add(globset::Glob::new(glob)?);
        }

        builder.build()?
    };

    config::load()?
        .into_par_iter()
        .map(|s: Spec| -> Result<(), Error> {
            let dest = s.url.local_dir()?;
            if !Path::new(dest).exists() {
                return Ok(());
            }
            let repo = git2::Repository::open(dest)?;
            grep_in(pattern, dest, s.url.provider().as_ref(), &globs, &repo)?;
            Ok(())
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

fn grep_in(
    pattern: &str,
    prefix: &str,
    provider: Option<&Provider>,
    globs: &globset::GlobSet,
    repo: &git2::Repository,
) -> Result<(), Error> {
    let matcher = RegexMatcher::new(pattern)?;
    let tree_obj = repo
        .revparse_single("origin/REMOTE_HEAD")
        .with_context(|| anyhow!("looking in {:?}", prefix))?
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

        if !globs.is_empty() && !globs.is_match(&path) {
            return git2::TreeWalkResult::Ok;
        }

        let status = Searcher::new().search_slice(
            &matcher,
            content,
            Lossy(|lnum, line| {
                let path = provider
                    .map(|p| href(&path, &p.html_browse_path(None, &path, Some(lnum))))
                    .unwrap_or_else(|| path.to_string());
                println!("{}/{} {}: {}", prefix, path, lnum, line.trim_end());
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
