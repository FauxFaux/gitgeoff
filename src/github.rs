use std::str::FromStr;

use failure::bail;
use failure::err_msg;
use failure::Error;
use hyperx::header::{Header, RelationType};
use reqwest::Method;
use serde_json::Value;
use url::Url;

pub fn flatten(pages: Vec<Value>) -> Result<Vec<Value>, Error> {
    let mut ret = Vec::with_capacity(100);
    for page in pages {
        ret.extend_from_slice(page.as_array().ok_or_else(|| err_msg("page wasn't list"))?);
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
