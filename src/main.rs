use std::env;

use failure::err_msg;
use failure::Error;
use failure::ResultExt;

mod github;

fn main() -> Result<(), Error> {
    let token = env::var("GH_TOKEN").with_context(|_| err_msg("reading GH_TOKEN from env"))?;

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

    let repos = github::all_pages(
        &format!(
            "https://api.github.com/orgs/{}/repos",
            matches.value_of("org").unwrap()
        ),
        &token,
    )?;

    for repo in repos {
        println!("{:?}", repo);
    }

    Ok(())
}
