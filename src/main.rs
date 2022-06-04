use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

mod cache;
mod config;
mod git;
mod git_url;
#[cfg(github)]
mod github;
mod grep;
mod status;
mod infect;

use cache::Cache;
use config::Spec;

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
                .short('t')
                .value_name("tags")
                .required(false)
                .multiple(true)
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("status")
                .about("Show the status of all child repos")
                .arg(Arg::with_name("update").long("update").short('u')),
        )
        .subcommand(
            SubCommand::with_name("grep")
                .about("Search for text in all child repos")
                .arg(Arg::with_name("pattern").required(true))
                .arg(Arg::with_name("globs").multiple(true)),
        )
        .subcommand(SubCommand::with_name("infect")
            .about("Add .git/config gitgeoff depends upon"))
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    match matches.subcommand() {
        Some(("status", args)) => {
            status::status(args.is_present("update"))?;
        }
        Some(("grep", args)) => {
            let pattern = args.value_of("pattern").expect("required");
            let globs = args
                .values_of("globs")
                .map(|v| v.collect::<Vec<&str>>())
                .unwrap_or_default();
            grep::grep(pattern, &globs)?;
        }
        Some(("infect", _)) => {
            infect::infect()?;
        }
        Some(("update", _args)) =>
        {
            #[cfg(never)]
            git::clone_or_fetch(&src, &dest)
                .with_context(|| anyhow!("ensure {:?} -> {:?}", src, dest))
        }
        Some((unknown_command, _args)) => unreachable!("unknown command: {:?}", unknown_command),
        _ => unreachable!("subcommand required"),
    }

    Ok(())
}
