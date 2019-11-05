use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use failure::err_msg;
use failure::format_err;
use failure::Error;
use failure::ResultExt;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

mod cache;
mod config;
mod git;
#[cfg(github)]
mod github;
mod grep;

use cache::Cache;
use config::Spec;
use git2::ErrorClass::Repository;

fn main() -> Result<(), Error> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cache = Cache::new()?;

    use clap::Arg;
    use clap::SubCommand;
    let matches = clap::App::new(clap::crate_name!())
        .arg(
            Arg::with_name("tags")
                .long("tags")
                .short("t")
                .value_name("tags")
                .required(false)
                .multiple(true)
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("status").arg(Arg::with_name("update").long("update").short("u")),
        )
        .subcommand(SubCommand::with_name("grep").arg(Arg::with_name("pattern").required(true)))
        .setting(clap::AppSettings::SubcommandRequired)
        .get_matches();

    match matches.subcommand() {
        ("status", Some(args)) => {
            #[derive(PartialEq, Eq, Clone, Debug)]
            enum Status {
                Absent,
                Changes(Vec<String>, git::Variance),
                Clean,
            }
            let status: Vec<(Spec, Status)> = config::load()?
                .into_par_iter()
                .map(|spec| -> Result<_, Error> {
                    let dest = spec.local_dir()?;
                    let dest = Path::new(dest);
                    if !dest.exists() {
                        return Ok((spec, Status::Absent));
                    }
                    let repo = git2::Repository::open(dest)?;
                    if args.is_present("update") {
                        git::fetch_origin_default(&repo)?;
                    }
                    let variance = git::variance_from_origin_head(&repo)?;
                    let some_statuses = git::first_statuses(&repo)?;
                    if !some_statuses.is_empty() || variance != git::Variance::Equal {
                        Ok((spec, Status::Changes(some_statuses, variance)))
                    } else {
                        Ok((spec, Status::Clean))
                    }
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
        }
        ("grep", Some(args)) => {
            let pattern = args.value_of("pattern").expect("required");
            grep::grep(pattern)?;
        }
        ("update", _args) =>
        {
            #[cfg(never)]
            git::clone_or_fetch(&src, &dest)
                .with_context(|_| format_err!("ensure {:?} -> {:?}", src, dest))
        }
        (_, _) => unreachable!("subcommand required"),
    }

    Ok(())
}
