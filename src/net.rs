use std::{rc::Rc, time::Duration};

use anyhow::Result;
use serde::Deserialize;

use crate::{Context, Doc, DocMeta, RawDocMeta, Repo};

const TOKEN_KEY: &str = "X-Auth-Token";
const QUERY_LIMIT: (&str, i16) = ("limit", i16::MAX);

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

#[inline]
async fn cool(cx: &Context<'_>) {
    let (requests, i) = cx.limit.get();
    if requests < cx.config.limit {
        cx.limit.set((requests + 1, i));
    } else {
        tokio::time::sleep_until(tokio::time::Instant::from_std(i + Duration::from_secs(1))).await;
    }
}
