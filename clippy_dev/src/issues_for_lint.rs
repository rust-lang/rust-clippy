use crate::gather_all;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{
    blocking::{Client, Response},
    header,
};
use serde::Deserialize;
use std::env;

lazy_static! {
    static ref NEXT_PAGE_RE: Regex = Regex::new(r#"<(?P<link>[^;]+)>;\srel="next""#).unwrap();
}

#[derive(Debug, Deserialize)]
struct Issue {
    title: String,
    number: u32,
    body: String,
    pull_request: Option<PR>,
}

#[derive(Debug, Deserialize)]
struct PR {}

enum Error {
    Reqwest(reqwest::Error),
    Env(std::env::VarError),
    Http(header::InvalidHeaderValue),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Self::Env(err)
    }
}

impl From<header::InvalidHeaderValue> for Error {
    fn from(err: header::InvalidHeaderValue) -> Self {
        Self::Http(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Reqwest(err) => write!(fmt, "reqwest: {}", err),
            Self::Env(err) => write!(fmt, "env: {}", err),
            Self::Http(err) => write!(fmt, "http: {}", err),
        }
    }
}

pub fn run(name: &str, filter: &[u32]) {
    match open_issues() {
        Ok(issues) => {
            for (i, issue) in filter_issues(&issues, name, filter).enumerate() {
                if i == 0 {
                    println!("### `{}`\n", name);
                }
                println!("- [ ] #{} ({})", issue.number, issue.title)
            }
        },
        Err(err) => eprintln!("{}", err),
    }
}

pub fn run_all(filter: &[u32]) {
    match open_issues() {
        Ok(issues) => {
            let mut lint_names = gather_all().map(|lint| lint.name).collect::<Vec<_>>();
            lint_names.sort();
            for name in lint_names {
                let mut print_empty_line = false;
                for (i, issue) in filter_issues(&issues, &name, filter).enumerate() {
                    if i == 0 {
                        println!("### `{}`\n", name);
                        print_empty_line = true;
                    }
                    println!("- [ ] #{} ({})", issue.number, issue.title)
                }
                if print_empty_line {
                    println!();
                }
            }
        },
        Err(err) => eprintln!("{}", err),
    }
}

fn open_issues() -> Result<Vec<Issue>, Error> {
    let github_token = env::var("GITHUB_TOKEN")?;

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("token {}", github_token))?,
    );
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("ghost"));
    let client = Client::builder().default_headers(headers).build()?;

    let issues_base = "https://api.github.com/repos/rust-lang/rust-clippy/issues";

    let mut issues = vec![];
    let mut response = client
        .get(issues_base)
        .query(&[("per_page", "100"), ("state", "open"), ("direction", "asc")])
        .send()?;
    while let Some(link) = next_link(&response) {
        issues.extend(
            response
                .json::<Vec<Issue>>()?
                .into_iter()
                .filter(|i| i.pull_request.is_none()),
        );
        response = client.get(&link).send()?;
    }

    Ok(issues)
}

fn filter_issues<'a>(issues: &'a [Issue], name: &str, filter: &'a [u32]) -> impl Iterator<Item = &'a Issue> {
    let name = name.to_lowercase();
    let separated_name = name.chars().map(|c| if c == '_' { ' ' } else { c }).collect::<String>();
    let dash_separated_name = name.chars().map(|c| if c == '_' { '-' } else { c }).collect::<String>();

    issues.iter().filter(move |i| {
        let title = i.title.to_lowercase();
        let body = i.body.to_lowercase();
        !filter.contains(&i.number)
            && (title.contains(&name)
                || title.contains(&separated_name)
                || title.contains(&dash_separated_name)
                || body.contains(&name)
                || body.contains(&separated_name)
                || body.contains(&dash_separated_name))
    })
}

fn next_link(response: &Response) -> Option<String> {
    if let Some(links) = response.headers().get("Link").and_then(|l| l.to_str().ok()) {
        if let Some(cap) = NEXT_PAGE_RE.captures_iter(links).next() {
            return Some(cap["link"].to_string());
        }
    }

    None
}
