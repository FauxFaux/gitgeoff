use std::env;

use anyhow::Error;
use clap::ArgAction;

mod cache;
mod config;
mod git;
mod git_url;
#[cfg(github)]
mod github;
mod grep;
mod infect;
mod status;

use cache::Cache;

fn main() -> Result<(), Error> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let _cache = Cache::new()?;

    use clap::Arg;
    use clap::Command;
    let matches = clap::command!()
        .arg(
            Arg::new("tags")
                .long("tags")
                .short('t')
                .value_name("tags")
                .required(false)
                .num_args(..),
        )
        .subcommand(
            Command::new("status")
                .about("Show the status of all child repos")
                .arg(
                    Arg::new("update")
                        .long("update")
                        .short('u')
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("grep")
                .about("Search for text in all child repos")
                .arg(Arg::new("pattern").required(true))
                .arg(Arg::new("globs").num_args(1..)),
        )
        .subcommand(Command::new("infect").about("Add .git/config gitgeoff depends upon"))
        .subcommand_required(true)
        .get_matches();

    match matches.subcommand() {
        Some(("status", args)) => {
            status::status(args.get_flag("update"))?;
        }
        Some(("grep", args)) => {
            let pattern = args.get_one::<String>("pattern").expect("required");
            let globs = args
                .get_many::<String>("globs")
                .map(|v| v.into_iter().collect::<Vec<&String>>())
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
