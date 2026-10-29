//! Where a release is read from: `<source>/releases/download/<tag>/<file>` of an
//! `https` repository, or `<directory>/<tag>/<file>` of a local directory.
//!
//! An HTTPS request uses no proxy and follows at most [`REDIRECT_LIMIT`] redirects,
//! each to `https`: a release asset download answers with a redirect to its storage
//! host, which the host's provider HTTP client (no redirect at all) would refuse.
//! What is read is bounded, and what it is worth is decided by the index's digests,
//! not by the transport.
use super::{REDIRECT_LIMIT, Refusal};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

/// A release source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An `https` repository URL without credentials, query or fragment.
    Https(Url),
    /// A local directory, canonical.
    Directory(PathBuf),
}

impl Source {
    /// Reads `text` as a source: an `https` URL, or else a directory that exists.
    /// Any other scheme is [`Refusal::InvalidSource`]; a directory that does not
    /// exist is [`Refusal::NotFound`].
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        if text.is_empty() || text.contains('\0') {
            return Err(Refusal::InvalidSource);
        }
        if text.contains("://") {
            let url = Url::parse(text).map_err(|_| Refusal::InvalidSource)?;
            if url.scheme() != "https"
                || url.host_str().is_none_or(str::is_empty)
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.cannot_be_a_base()
            {
                return Err(Refusal::InvalidSource);
            }
            return Ok(Self::Https(url));
        }
        let directory = std::fs::canonicalize(text).map_err(|_| Refusal::NotFound)?;
        if !directory.is_dir() {
            return Err(Refusal::NotFound);
        }
        // The lock records the path as text.
        directory.to_str().ok_or(Refusal::InvalidSource)?;
        Ok(Self::Directory(directory))
    }

    /// The source as the lock records it: the URL, or the directory's canonical path.
    pub fn recorded(&self) -> String {
        match self {
            Self::Https(url) => url.as_str().trim_end_matches('/').to_owned(),
            Self::Directory(directory) => directory.to_string_lossy().into_owned(),
        }
    }
}

/// Reads release files from one source; an HTTPS source holds one client and one
/// runtime for every file of an install.
pub(crate) enum Fetcher {
    Https {
        base: Url,
        client: reqwest::Client,
        runtime: tokio::runtime::Runtime,
    },
    Directory(PathBuf),
}

impl Fetcher {
    pub(crate) fn new(source: &Source) -> Result<Self, Refusal> {
        match source {
            Source::Directory(directory) => Ok(Self::Directory(directory.clone())),
            Source::Https(url) => {
                // reqwest carries no crypto provider of its own; ring is the one this
                // workspace selects.
                let _ = rustls::crypto::ring::default_provider().install_default();
                let client = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::custom(|attempt| {
                        if attempt.previous().len() >= REDIRECT_LIMIT
                            || attempt.url().scheme() != "https"
                        {
                            attempt.error("redirect not followed")
                        } else {
                            attempt.follow()
                        }
                    }))
                    .connect_timeout(Duration::from_secs(10))
                    .timeout(Duration::from_secs(120))
                    .build()
                    .map_err(|_| Refusal::Unavailable)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| Refusal::Unavailable)?;
                Ok(Self::Https {
                    base: url.clone(),
                    client,
                    runtime,
                })
            }
        }
    }

    /// The bytes of `name` of the release `tag`, at most `limit` of them.
    pub(crate) fn read(&mut self, tag: &str, name: &str, limit: u64) -> Result<Vec<u8>, Refusal> {
        match self {
            Self::Directory(directory) => read_file(&directory.join(tag).join(name), limit),
            Self::Https {
                base,
                client,
                runtime,
            } => {
                let url = release_url(base, tag, name)?;
                runtime.block_on(download(client, url, limit))
            }
        }
    }
}

/// `<base>/releases/download/<tag>/<name>`, each a single path segment.
pub(crate) fn release_url(base: &Url, tag: &str, name: &str) -> Result<Url, Refusal> {
    let mut url = base.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| Refusal::InvalidSource)?;
        segments
            .pop_if_empty()
            .extend(["releases", "download", tag, name]);
    }
    Ok(url)
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, Refusal> {
    let file = std::fs::File::open(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => Refusal::NotFound,
        _ => Refusal::Unavailable,
    })?;
    let metadata = file.metadata().map_err(|_| Refusal::Unavailable)?;
    if !metadata.is_file() {
        return Err(Refusal::NotFound);
    }
    if metadata.len() > limit {
        return Err(Refusal::Unavailable);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Unavailable)?;
    if bytes.len() as u64 > limit {
        return Err(Refusal::Unavailable);
    }
    Ok(bytes)
}

async fn download(client: &reqwest::Client, url: Url, limit: u64) -> Result<Vec<u8>, Refusal> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| Refusal::Unavailable)?;
    match response.status() {
        reqwest::StatusCode::NOT_FOUND => return Err(Refusal::NotFound),
        status if !status.is_success() => return Err(Refusal::Unavailable),
        _ => {}
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(Refusal::Unavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Refusal::Unavailable)? {
        if bytes.len() as u64 + chunk.len() as u64 > limit {
            return Err(Refusal::Unavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
