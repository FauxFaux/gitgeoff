use std::str::FromStr;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use hyperx::header::{Header, RelationType};
use reqwest::Method;
use serde_json::Value;
use url::Url;

pub fn flatten(pages: Vec<Value>) -> Result<Vec<Value>, Error> {
    let mut ret = Vec::with_capacity(100);
    for page in pages {
        ret.extend_from_slice(page.as_array().ok_or_else(|| anyhow!("page wasn't list"))?);
    }
    Ok(ret)
}

pub fn all_pages(base_url: &str, token: &str) -> Result<Vec<Value>, Error> {
    let client = reqwest::Client::new();

    let mut pages = Vec::with_capacity(10);

    let mut base_url = url::Url::from_str(base_url)?;
    base_url.set_query(Some(&match base_url.query() {
        Some(query) => format!("{}&per_page=100", query),
        None => format!("per_page=100"),
    }));

    let mut url = base_url;

    loop {
        let mut resp = client
            .request(Method::GET, url)
            .basic_auth(token, Some(""))
            .send()?;

        if !resp.status().is_success() {
            bail!("request for {:?} failed: {:?}", resp.url(), resp.status());
        }

        pages.push(resp.json()?);

        match next_page(&resp)? {
            Some(page) => url = page,
            None => break,
        }
    }

    Ok(pages)
}

fn next_page(resp: &reqwest::Response) -> Result<Option<Url>, Error> {
    match hyperx::header::Link::parse_header(
        &resp.headers().get_all(hyperx::header::Link::header_name()),
    )?
    .values()
    .into_iter()
    .find(|value| value.rel() == Some(&[RelationType::Next]))
    .map(|value| value.link())
    {
        Some(url) => Ok(Some(Url::from_str(url)?)),
        None => Ok(None),
    }
}

type IsoDate = chrono::DateTime<chrono::Utc>;

#[derive(serde_derive::Deserialize, Clone)]
pub(crate) struct Repo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub owner: Value,
    pub description: Option<String>,
    pub fork: bool,

    pub ssh_url: String,

    pub created_at: IsoDate,
    pub updated_at: IsoDate,
    pub pushed_at: IsoDate,

    // ??
    pub size: u64,

    pub stargazers_count: u64,
    pub watchers_count: u64,

    pub has_issues: bool,
    pub open_issues_count: u64,
    pub open_issues: u64,
    pub forks_count: u64,

    pub archived: bool,
    pub disabled: bool,

    // master
    pub default_branch: String,

    pub permissions: Permissions,
}

#[derive(serde_derive::Deserialize, Clone)]
pub struct Permissions {
    pub admin: bool,
    pub push: bool,
    pub pull: bool,
}

pub fn write_github(token: &String, cache: super::cache::Cache, org: &&str) -> Result<(), Error> {
    let repos = all_pages(
        &format!("https://api.github.com/orgs/{}/repos", org),
        &token,
    )?;

    let repos = flatten(repos)?;

    let repos_json = cache.meta_github_org(org)?.join("repos.json");

    let mut temp = tempfile_fast::Sponge::new_for(repos_json)?;
    serde_json::to_writer(&mut temp, &repos)?;
    temp.commit()?;

    Ok(())
}
