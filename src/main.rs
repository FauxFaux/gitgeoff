use std::env;

use failure::err_msg;
use failure::Error;
use failure::ResultExt;

mod cache;
mod github;

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let token = env::var("GH_TOKEN").with_context(|_| err_msg("reading GH_TOKEN from env"))?;

    let cache = cache::Cache::new()?;

    use clap::Arg;
    let matches = clap::App::new(clap::crate_name!())
        .arg(
            Arg::with_name("org")
                .long("org")
                .value_name("ORG")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let org = matches.value_of("org").unwrap();

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
