use std::path::Path;

use failure::format_err;
use failure::Error;
use failure::ResultExt;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use super::config;
use super::git;
use config::Spec;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Status {
    Absent,
    Changes(Vec<String>, git::Variance),
    Clean,
}

pub fn status(update: bool) -> Result<(), Error> {
    let status: Vec<(Spec, Status)> = config::load()?
        .into_par_iter()
        .map(|spec| -> Result<_, Error> {
            let dest = spec.local_dir()?;
            let dest = Path::new(dest);
            if !dest.exists() {
                return Ok((spec, Status::Absent));
            }
            let repo = git2::Repository::open(dest)?;
            if update {
                git::fetch_origin_default(&repo)?;
            }
            let status = find_variance(&repo)
                .with_context(|_| format_err!("finding status of {:?}", dest))?;
            Ok((spec, status))
        })
        .collect::<Result<_, _>>()?;

    println!(
        "absent: {}",
        status
            .iter()
            .filter_map(|(spec, status)| match status {
                Status::Absent => spec.local_dir().ok(),
                _ => None,
            })
            .collect::<Vec<&str>>()
            .join(", ")
    );

    println!(
        "clean: {}",
        status
            .iter()
            .filter_map(|(spec, status)| match status {
                Status::Clean => spec.local_dir().ok(),
                _ => None,
            })
            .collect::<Vec<&str>>()
            .join(", ")
    );

    for (spec, stat) in status {
        let (changes, variance) = match stat {
            Status::Changes(changes, variance) => (changes, variance),
            _ => continue,
        };
        let suffix = if changes.len() > 2 { ", ..." } else { "" };
        println!(
            "{}: ({:?}) {}{}",
            spec.local_dir()?,
            variance,
            changes.into_iter().take(2).collect::<Vec<_>>().join(", "),
            suffix
        );
    }

    Ok(())
}

fn find_variance(repo: &git2::Repository) -> Result<Status, Error> {
    let variance = git::variance_from_origin_head(&repo)?;
    let some_statuses = git::first_statuses(&repo)?;
    Ok(
        if !some_statuses.is_empty() || variance != git::Variance::Equal {
            Status::Changes(some_statuses, variance)
        } else {
            Status::Clean
        },
    )
}
