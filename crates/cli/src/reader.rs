use std::sync::Arc;

use bytes::Bytes;
use fast_pull::{RandReader, SeqReader, reqwest::ReqwestReader};
use futures::TryStream;
use reqwest::{
    ClientBuilder, Proxy,
    header::{HeaderMap, HeaderValue},
};
use url::Url;

pub fn build_client(
    headers: &HeaderMap,
    proxy: &Option<String>,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut client = ClientBuilder::new()
        .default_headers(headers.clone())
        .http2_adaptive_window(true)
        .brotli(true)
        .gzip(true)
        .deflate(true)
        .zstd(true);
    if let Some(ref proxy) = *proxy {
        client = client.proxy(Proxy::all(proxy)?);
    }
    let client = client.build()?;
    Ok(client)
}

pub struct FastDownReader {
    inner: ReqwestReader,
    headers: Arc<HeaderMap<HeaderValue>>,
    proxy: Arc<Option<String>>,
    url: Arc<Url>,
}

impl FastDownReader {
    pub fn new(
        url: Url,
        headers: HeaderMap<HeaderValue>,
        proxy: Option<String>,
    ) -> Result<Self, reqwest::Error> {
        let client = build_client(&headers, &proxy)?;
        Ok(Self {
            inner: ReqwestReader::new(url.clone(), client),
            headers: Arc::new(headers),
            proxy: Arc::new(proxy),
            url: Arc::new(url),
        })
    }
}

impl Clone for FastDownReader {
    fn clone(&self) -> Self {
        let client = build_client(&self.headers, &self.proxy).unwrap();
        Self {
            inner: ReqwestReader::new(self.url.as_ref().clone(), client),
            headers: self.headers.clone(),
            proxy: self.proxy.clone(),
            url: self.url.clone(),
        }
    }
}

impl RandReader for FastDownReader {
    type Error = reqwest::Error;
    fn read(
        &mut self,
        range: &fast_pull::ProgressEntry,
    ) -> impl TryStream<Ok = Bytes, Error = Self::Error> + Send + Unpin {
        RandReader::read(&mut self.inner, range)
    }
}

impl SeqReader for FastDownReader {
    type Error = reqwest::Error;
    fn read(&mut self) -> impl TryStream<Ok = Bytes, Error = Self::Error> + Send + Unpin {
        SeqReader::read(&mut self.inner)
    }
}
