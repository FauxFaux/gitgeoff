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
            #[derive(PartialEq, Eq, Copy, Clone)]
            enum Status {
                Absent,
                Changes,
                Clean,
            }
            let status = config::load()?
                .into_par_iter()
                .map(|spec| -> Result<_, Error> {
                    let dest = spec.local_dir()?;
                    let dest = Path::new(dest);
                    if !dest.exists() {
                        return Ok(Status::Absent);
                    }
                    if git::status(dest)? {
                        Ok(Status::Changes)
                    } else {
                        Ok(Status::Clean)
                    }
                })
                .collect::<Result<Vec<Status>, _>>()?;

            println!(
                "{} changed, {} absent, {} clean",
                status.iter().filter(|&&e| e == Status::Changes).count(),
                status.iter().filter(|&&e| e == Status::Absent).count(),
                status.iter().filter(|&&e| e == Status::Clean).count(),
            );
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
