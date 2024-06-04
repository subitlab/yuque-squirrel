use std::{
    cell::{Cell, RefCell},
    fmt::{Debug, Display},
    path::PathBuf,
    rc::Rc,
    time::Instant,
};

use anyhow::Result;
use clap::Parser;
use reqwest::Url;
use serde::{Deserialize, Serialize};

mod config;
mod net;
mod store;

use config::Config;
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::store::MainMetadata;

/// The global context.
#[derive(Debug, Clone, Copy)]
struct Context<'a> {
    config: &'a Config,
    h2_client: &'a reqwest::Client,

    limit: &'a Cell<(usize, Instant)>,
    meta: &'a RefCell<MainMetadata>,
}

impl Context<'_> {
    /// Constructs a [`Url`] with the given suffix.
    #[inline]
    fn url<T: AsRef<str>>(&self, suffix: T) -> Result<Url> {
        Url::parse(&format!("{}{}", self.config.host, suffix.as_ref())).map_err(Into::into)
    }

    #[inline]
    fn uri_path(&self) -> UriPath<'_> {
        UriPath { cx: self }
    }
}

#[derive(Debug)]
struct UriPath<'a> {
    cx: &'a Context<'a>,
}

impl Display for UriPath<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "/{}/{}",
            self.cx.config.target.ty, self.cx.config.target.login
        )
    }
}

/// A repository structure, compatible with the API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repo {
    id: i64,
    slug: String,
    name: String,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawDocMeta {
    id: i64,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct DocMeta<'repo> {
    repo: &'repo Repo,
    raw: Rc<RawDocMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Doc {
    id: i64,
    #[serde(rename = "type")]
    ty: String,
    slug: String,
    title: String,
    book_id: i64,
    description: String,
    format: String,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,

    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    body_sheet: Option<String>,
    #[serde(default)]
    body_html: Option<String>,
    #[serde(default)]
    body_lake: Option<String>,
}

/// A secret Yuque token.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct Token(String);

impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "*****")
    }
}

impl TryFrom<&Token> for reqwest::header::HeaderValue {
    type Error = reqwest::header::InvalidHeaderValue;

    #[inline]
    fn try_from(value: &Token) -> Result<Self, Self::Error> {
        Self::from_str(&value.0)
    }
}

fn main() -> Result<()> {
    /// Yuque backup utilities.
    #[derive(Parser)]
    #[command(version, about, long_about = None)]
    struct Cli {
        /// Path the backup directory is.
        path: Option<PathBuf>,

        /// Configuration file.
        #[arg(short, value_name = "FILE")]
        config: PathBuf,
    }

    let Cli { path, config } = Cli::parse();
    let path = path.unwrap_or_else(|| PathBuf::from(r"./"));
    let meta_path = path.join("metadata.json");
    let t_now = OffsetDateTime::now_utc();
    let backup_path =
        path.join(t_now.format(&time::format_description::well_known::Iso8601::DATE_TIME)?);

    if !backup_path.try_exists()? {
        std::fs::create_dir_all(&backup_path)?;
    }

    let config: Config = serde_json::from_reader(std::fs::File::open(config)?)?;

    let h2_client = reqwest::Client::new();
    let limit = Cell::new((0usize, Instant::now()));
    let main_meta = RefCell::new(
        std::fs::File::open(&meta_path)
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
            .unwrap_or_default(),
    );

    let cx = Context {
        config: &config,
        h2_client: &h2_client,
        limit: &limit,
        meta: &main_meta,
    };

    let mut rt = tokio::runtime::Builder::new_current_thread();
    rt.enable_all();
    let rt = rt.build()?;

    rt.block_on(async {
        let repos = net::repos(cx).await?;
        for chunk in repos.chunks(16) {
            cx.meta
                .borrow_mut()
                .books
                .extend(repos.iter().cloned().map(|r| (r.id, r)));
            let _ = futures::future::join_all(chunk.iter().map(|repo| async {
                let metas = net::doc_metas(cx, repo).await?;
                let backup_path = &backup_path;
                for meta_chunk in metas.chunks(16) {
                    let _ = futures::future::join_all(
                        meta_chunk
                            .iter()
                            .filter(|m| cx.meta.borrow().needs_backup(m))
                            .cloned()
                            .map(|m| async move {
                                let doc = net::doc(cx, m.clone()).await.inspect_err(|err| {
                                    eprintln!("error obtaining document: {}", err)
                                })?;
                                let mut file = tokio::fs::File::create_new(
                                    backup_path.join(format!("doc{}.json", m.raw.id)),
                                )
                                .await?;
                                file.write_all(&serde_json::to_vec_pretty(&doc)?).await?;
                                cx.meta.borrow_mut().track_backup(&m);
                                Result::<_, anyhow::Error>::Ok(())
                            }),
                    )
                    .await;
                }
                Result::<_, anyhow::Error>::Ok(())
            }))
            .await;
        }
        Result::<_, anyhow::Error>::Ok(())
    })?;

    std::fs::write(meta_path, serde_json::to_vec_pretty(&main_meta)?)?;
    Ok(())
}
