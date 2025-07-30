use content_disposition;
use reqwest::{
    Client, IntoUrl, StatusCode, Url,
    header::{self, HeaderMap},
};
use sanitize_filename;

#[derive(Debug, Clone)]
pub struct UrlInfo {
    pub file_size: u64,
    pub file_name: String,
    pub supports_range: bool,
    pub can_fast_download: bool,
    pub final_url: Url,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

fn get_file_size(headers: &HeaderMap, status: &StatusCode) -> u64 {
    if *status == StatusCode::PARTIAL_CONTENT {
        headers
            .get(header::CONTENT_RANGE)
            .and_then(|hv| hv.to_str().ok())
            .and_then(|s| s.rsplit('/').next())
            .and_then(|total| total.parse().ok())
            .unwrap_or(0)
    } else {
        headers
            .get(header::CONTENT_LENGTH)
            .and_then(|hv| hv.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
}

fn get_header_str(headers: &HeaderMap, header_name: &header::HeaderName) -> Option<String> {
    headers
        .get(header_name)
        .and_then(|hv| hv.to_str().ok())
        .map(String::from)
}

fn get_filename(headers: &HeaderMap, final_url: &Url) -> String {
    let from_disposition = headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| content_disposition::parse_content_disposition(s).filename_full())
        .filter(|s| !s.trim().is_empty());

    let from_url = final_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .and_then(|s| urlencoding::decode(s).ok())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let raw_name = from_disposition
        .or(from_url)
        .unwrap_or_else(|| final_url.to_string());

    sanitize_filename::sanitize_with_options(
        &raw_name,
        sanitize_filename::Options {
            windows: true,
            truncate: true,
            replacement: "_",
        },
    )
}

pub trait Prefetch {
    fn prefetch(
        &self,
        url: impl IntoUrl + Send,
    ) -> impl Future<Output = Result<UrlInfo, reqwest::Error>> + Send;
}

impl Prefetch for Client {
    async fn prefetch(&self, url: impl IntoUrl + Send) -> Result<UrlInfo, reqwest::Error> {
        let url = url.into_url()?;
        let resp = self.head(url.clone()).send().await?;
        let resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(_) => return prefetch_fallback(url, self).await,
        };

        let status = resp.status();
        let final_url = resp.url();

        let resp_headers = resp.headers();
        let file_size = get_file_size(resp_headers, &status);

        let supports_range = match resp.headers().get(header::ACCEPT_RANGES) {
            Some(accept_ranges) => accept_ranges
                .to_str()
                .ok()
                .map(|v| v.split(' '))
                .and_then(|supports| supports.into_iter().find(|&ty| ty == "bytes"))
                .is_some(),
            None => return prefetch_fallback(url, self).await,
        };

        Ok(UrlInfo {
            final_url: final_url.clone(),
            file_name: get_filename(resp_headers, final_url),
            file_size,
            supports_range,
            can_fast_download: file_size > 0 && supports_range,
            etag: get_header_str(resp_headers, &header::ETAG),
            last_modified: get_header_str(resp_headers, &header::LAST_MODIFIED),
        })
    }
}

async fn prefetch_fallback(url: Url, client: &Client) -> Result<UrlInfo, reqwest::Error> {
    let resp = client
        .get(url)
        .header(header::RANGE, "bytes=0-")
        .send()
        .await?
        .error_for_status()?;
    let status = resp.status();
    let final_url = resp.url();

    let resp_headers = resp.headers();
    let file_size = get_file_size(resp_headers, &status);
    let supports_range = status == StatusCode::PARTIAL_CONTENT;
    Ok(UrlInfo {
        final_url: final_url.clone(),
        file_name: get_filename(resp_headers, final_url),
        file_size,
        supports_range,
        can_fast_download: file_size > 0 && supports_range,
        etag: get_header_str(resp_headers, &header::ETAG),
        last_modified: get_header_str(resp_headers, &header::LAST_MODIFIED),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redirect_and_content_range() {
        let mut server = mockito::Server::new_async().await;

        let mock_redirect = server
            .mock("GET", "/redirect")
            .with_status(301)
            .with_header("Location", "/real-file.txt")
            .create_async()
            .await;

        let mock_file = server
            .mock("GET", "/real-file.txt")
            .with_status(206)
            .with_header("Content-Range", "bytes 0-1023/2048")
            .with_body(vec![0; 1024])
            .create_async()
            .await;

        let client = Client::new();
        let url_info = client
            .prefetch(&format!("{}/redirect", server.url()))
            .await
            .expect("Request should succeed");

        assert_eq!(url_info.file_size, 2048);
        assert_eq!(url_info.file_name, "real-file.txt");
        assert_eq!(
            url_info.final_url.as_str(),
            format!("{}/real-file.txt", server.url())
        );
        assert!(url_info.supports_range);

        mock_redirect.assert_async().await;
        mock_file.assert_async().await;
    }

    #[tokio::test]
    async fn test_content_range_priority() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/file")
            .with_status(206)
            .with_header("Content-Range", "bytes 0-1023/2048")
            .create_async()
            .await;

        let client = Client::new();
        let url_info = client
            .prefetch(&format!("{}/file", server.url()))
            .await
            .expect("Request should succeed");

        assert_eq!(url_info.file_size, 2048);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_filename_sources() {
        let mut server = mockito::Server::new_async().await;

        // Test Content-Disposition source
        let mock1 = server
            .mock("GET", "/test1")
            .with_header("Content-Disposition", "attachment; filename=\"test.txt\"")
            .create_async()
            .await;
        let url_info = Client::new()
            .prefetch(&format!("{}/test1", server.url()))
            .await
            .unwrap();
        assert_eq!(url_info.file_name, "test.txt");
        mock1.assert_async().await;

        // Test URL path source
        let mock2 = server.mock("GET", "/test2/file.pdf").create_async().await;
        let url_info = Client::new()
            .prefetch(&format!("{}/test2/file.pdf", server.url()))
            .await
            .unwrap();
        assert_eq!(url_info.file_name, "file.pdf");
        mock2.assert_async().await;

        // Test sanitization
        let mock3 = server
      .mock("GET", "/test3")
      .with_header(
        "Content-Disposition",
        "attachment; filename*=UTF-8''%E6%82%AA%E3%81%84%3C%3E%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%3F%E5%90%8D.txt",
      )
      .create_async()
      .await;
        let url_info = Client::new()
            .prefetch(&format!("{}/test3", server.url()))
            .await
            .unwrap();
        assert_eq!(url_info.file_name, "悪い__ファイル_名.txt");
        mock3.assert_async().await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut server = mockito::Server::new_async().await;
        let mock1 = server
            .mock("GET", "/404")
            .with_status(404)
            .create_async()
            .await;

        let client = Client::new();

        match client.prefetch(&format!("{}/404", server.url())).await {
            Ok(info) => panic!("404 status code should not success: {info:?}"),
            Err(err) => {
                assert!(err.is_status(), "should be error about status code");
                assert_eq!(err.status(), Some(StatusCode::NOT_FOUND));
            }
        }

        mock1.assert_async().await;
    }
}
