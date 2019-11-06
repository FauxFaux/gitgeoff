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
mod git_url;
#[cfg(github)]
mod github;
mod grep;
mod status;

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
            status::status(args.is_present("update"))?;
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
