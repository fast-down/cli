use bytes::Bytes;
use fast_pull::{RandPuller, SeqPuller, reqwest::ReqwestPuller};
use futures::TryStream;
use reqwest::{
    ClientBuilder, Proxy,
    header::{HeaderMap, HeaderValue},
};
use std::sync::Arc;
use url::Url;

pub fn build_client(
    headers: &HeaderMap,
    proxy: &Option<String>,
    accept_invalid_certs: bool,
    accept_invalid_hostnames: bool,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut client = ClientBuilder::new()
        .default_headers(headers.clone())
        .danger_accept_invalid_certs(accept_invalid_certs)
        .danger_accept_invalid_hostnames(accept_invalid_hostnames)
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

pub struct FastDownPuller {
    inner: ReqwestPuller,
    headers: Arc<HeaderMap<HeaderValue>>,
    proxy: Arc<Option<String>>,
    url: Arc<Url>,
    multiplexing: bool,
    accept_invalid_certs: bool,
    accept_invalid_hostnames: bool,
}

impl FastDownPuller {
    pub fn new(
        url: Url,
        headers: HeaderMap<HeaderValue>,
        proxy: Option<String>,
        multiplexing: bool,
        accept_invalid_certs: bool,
        accept_invalid_hostnames: bool,
    ) -> Result<Self, reqwest::Error> {
        let client = build_client(
            &headers,
            &proxy,
            accept_invalid_certs,
            accept_invalid_hostnames,
        )?;
        Ok(Self {
            inner: ReqwestPuller::new(url.clone(), client),
            headers: Arc::new(headers),
            proxy: Arc::new(proxy),
            url: Arc::new(url),
            multiplexing,
            accept_invalid_certs,
            accept_invalid_hostnames,
        })
    }
}

impl Clone for FastDownPuller {
    fn clone(&self) -> Self {
        if self.multiplexing {
            Self {
                inner: self.inner.clone(),
                headers: self.headers.clone(),
                proxy: self.proxy.clone(),
                url: self.url.clone(),
                multiplexing: self.multiplexing,
                accept_invalid_certs: self.accept_invalid_certs,
                accept_invalid_hostnames: self.accept_invalid_hostnames,
            }
        } else {
            let client = build_client(
                &self.headers,
                &self.proxy,
                self.accept_invalid_certs,
                self.accept_invalid_hostnames,
            )
            .unwrap();
            Self {
                inner: ReqwestPuller::new(self.url.as_ref().clone(), client),
                headers: self.headers.clone(),
                proxy: self.proxy.clone(),
                url: self.url.clone(),
                multiplexing: self.multiplexing,
                accept_invalid_certs: self.accept_invalid_certs,
                accept_invalid_hostnames: self.accept_invalid_hostnames,
            }
        }
    }
}

impl RandPuller for FastDownPuller {
    type Error = reqwest::Error;
    fn pull(
        &mut self,
        range: &fast_pull::ProgressEntry,
    ) -> impl TryStream<Ok = Bytes, Error = Self::Error> + Send + Unpin {
        RandPuller::pull(&mut self.inner, range)
    }
}

impl SeqPuller for FastDownPuller {
    type Error = reqwest::Error;
    fn pull(&mut self) -> impl TryStream<Ok = Bytes, Error = Self::Error> + Send + Unpin {
        SeqPuller::pull(&mut self.inner)
    }
}
