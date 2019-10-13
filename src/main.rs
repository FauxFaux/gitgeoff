use std::env;
use std::fs;
use std::io;

use failure::err_msg;
use failure::format_err;
use failure::Error;
use failure::ResultExt;

mod cache;
mod git;
mod github;

use cache::Cache;

fn main() -> Result<(), Error> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let token = env::var("GH_TOKEN").with_context(|_| err_msg("reading GH_TOKEN from env"))?;

    let cache = Cache::new()?;

    use clap::Arg;
    use clap::SubCommand;
    let matches = clap::App::new(clap::crate_name!())
        .arg(
            Arg::with_name("org")
                .long("org")
                .value_name("ORG")
                .required(true)
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("update"))
        .subcommand(SubCommand::with_name("update-github"))
        .setting(clap::AppSettings::SubcommandRequired)
        .get_matches();

    let org = matches.value_of("org").unwrap();

    // config:
    //   foo:
    //     github:
    //       token: FOO (from env GH_TOKEN_FOO, default GH_TOKEN)
    //       org: foo (default self)

    // currently assuming foo == foo

    match matches.subcommand() {
        ("update-github", _args) => {
            write_github(&token, cache, &org)?;
        }
        ("update", _args) => {
            let repos: Vec<github::Repo> = serde_json::from_reader(io::BufReader::new(
                fs::File::open(cache.meta_github_org(org)?.join("repos.json"))?,
            ))?;

            for repo in repos {
                if repo.archived {
                    continue;
                }
                let dest = cache.repo_bare(org, &repo.name)?;
                let src = &repo.ssh_url;
                git::clone_or_fetch(src, &dest)
                    .with_context(|_| format_err!("ensure {:?} -> {:?}", src, dest))?;
            }
        }
        (_, _) => unreachable!("subcommand required"),
    }

    Ok(())
}

fn write_github(token: &String, cache: Cache, org: &&str) -> Result<(), Error> {
    let repos = github::all_pages(
        &format!("https://api.github.com/orgs/{}/repos", org),
        &token,
    )?;

    let repos = github::flatten(repos)?;

    let repos_json = cache.meta_github_org(org)?.join("repos.json");

    let mut temp = tempfile_fast::Sponge::new_for(repos_json)?;
    serde_json::to_writer(&mut temp, &repos)?;
    temp.commit()?;

    Ok(())
}
