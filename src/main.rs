use std::env;
use std::fs;
use std::io;
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

use cache::Cache;

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
        .subcommand(SubCommand::with_name("status"))
        .setting(clap::AppSettings::SubcommandRequired)
        .get_matches();

    match matches.subcommand() {
        ("status", _args) => {
            let repos = config::load()?;

            let mut work = Vec::with_capacity(repos.len());

            for repo in repos {
                let src = repo.url;
                let dest = src
                    .path_segments()
                    .ok_or_else(|| err_msg("empty url"))?
                    .last()
                    .ok_or_else(|| err_msg("empty path in url"))?;

                work.push((src.as_str().to_string(), PathBuf::from(dest)));
            }

            work.into_par_iter()
                .map(|(src, dest)| {
                    git::clone_or_fetch(&src, &dest)
                        .with_context(|_| format_err!("ensure {:?} -> {:?}", src, dest))
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
        (_, _) => unreachable!("subcommand required"),
    }

    Ok(())
}
