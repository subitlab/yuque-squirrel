use std::{
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::{Context, Doc, DocMeta, RawDocMeta, Repo};

const TOKEN_KEY: &str = "X-Auth-Token";
const QUERY_LIMIT: (&str, &str) = ("limit", "100");
const USER_AGENT_KEY: &str = "User-Agent";
const USER_AGENT_VALUE: &str = "User-Agent Mozilla/5.0";

#[derive(Deserialize)]
struct ResponseObj<T> {
    data: T,
}

/// Gets repositories of the target.
pub async fn repos(cx: Context<'_>) -> Result<Vec<Repo>> {
    cool(&cx).await;

    let url = cx.url(format!("/api/v2{}/repos", cx.uri_path()))?;
    cx.h2_client
        .get(url)
        .header(TOKEN_KEY, &cx.config.token)
        .header(USER_AGENT_KEY, USER_AGENT_VALUE)
        .query(&[QUERY_LIMIT])
        .send()
        .await?
        .json::<ResponseObj<Vec<Repo>>>()
        .await
        .map(|obj| obj.data)
        .map_err(Into::into)
}

/// Gets document details of the given id and [`Repo`].
pub async fn doc(cx: Context<'_>, meta: DocMeta<'_>) -> Result<Doc> {
    cool(&cx).await;

    let url = cx.url(format!(
        "/api/v2/repos/{}/docs/{}",
        meta.repo.id, meta.raw.id
    ))?;
    cx.h2_client
        .get(url)
        .header(TOKEN_KEY, &cx.config.token)
        .header(USER_AGENT_KEY, USER_AGENT_VALUE)
        .send()
        .await?
        .json::<ResponseObj<Doc>>()
        .await
        .map(|obj| obj.data)
        .map_err(Into::into)
}

/// Gets document metadatas of the given [`Repo`].
pub async fn doc_metas<'repo>(cx: Context<'_>, repo: &'repo Repo) -> Result<Vec<DocMeta<'repo>>> {
    cool(&cx).await;

    let url = cx.url(format!("/api/v2/repos/{}/docs", repo.id))?;
    cx.h2_client
        .get(url)
        .header(TOKEN_KEY, &cx.config.token)
        .header(USER_AGENT_KEY, USER_AGENT_VALUE)
        .query(&[QUERY_LIMIT])
        .send()
        .await?
        .json::<ResponseObj<Vec<RawDocMeta>>>()
        .await
        .map(|obj| {
            obj.data
                .into_iter()
                .map(|meta| DocMeta {
                    repo,
                    raw: Rc::new(meta),
                })
                .collect()
        })
        .map_err(Into::into)
}

pub async fn resource(cx: Context<'_>, url: reqwest::Url, path: &Path) -> Result<()> {
    let mut stream = cx
        .h2_client
        .get(url)
        .header(TOKEN_KEY, &cx.config.token)
        .header(USER_AGENT_KEY, USER_AGENT_VALUE)
        .send()
        .await?
        .bytes_stream();
    let mut file = tokio::fs::File::create_new(path).await?;
    while let Some(mut chunk) = stream.try_next().await? {
        file.write_all_buf(&mut chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

#[inline]
async fn cool(cx: &Context<'_>) {
    let (requests, i) = cx.limit.get();
    if requests < cx.config.limit {
        cx.limit.set((requests + 1, i));
    } else {
        tokio::time::sleep_until(tokio::time::Instant::from_std(i + Duration::from_secs(1))).await;
        if cx.limit.get().1 == i {
            cx.limit.set((1, Instant::now()));
        }
    }
}
