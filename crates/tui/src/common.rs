use std::sync::Arc;
use reqwest::header::HeaderMap;

#[derive(Clone)]
pub struct ClientOptions(Arc<ClientOptionsInner>);

pub struct ClientOptionsInner {
    pub headers: HeaderMap,
}

impl ClientOptions {
    pub fn headers(&self) -> HeaderMap {
        self.0.headers.clone()
    }
}