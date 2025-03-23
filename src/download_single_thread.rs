use crate::{download_progress::DownloadProgress, get_url_info::UrlInfo};
use color_eyre::eyre::Result;
use reqwest::blocking::Client;
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    thread::{self, JoinHandle},
};

pub fn download_single_thread(
    file: File,
    client: Client,
    info: UrlInfo,
) -> Result<(
    crossbeam_channel::Receiver<DownloadProgress>,
    JoinHandle<()>,
)> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let (tx_write, rx_write) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        let mut downloaded = 0;
        let mut response = client.get(info.final_url).send().unwrap();
        let mut buffer = [0u8; 8 * 1024];
        loop {
            let len = response.read(&mut buffer).unwrap();
            if len == 0 {
                break;
            }
            tx.send(DownloadProgress::new(downloaded, downloaded + len - 1))
                .unwrap();
            downloaded += len;
            tx_write.send(buffer[..len].to_vec()).unwrap();
        }
    });
    let handle = thread::spawn(move || {
        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
        for bytes in rx_write {
            writer.write_all(&bytes).unwrap();
        }
        writer.flush().unwrap();
    });
    Ok((rx, handle))
}

#[cfg(test)]
mod tests {
    use crate::progresses_size::ProgressesSize;

    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_downloads_small_file_correctly() {
        // 测试 9B 小文件
        let mock_body = b"test data";
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(mock_body)
            .create();

        let url_info = UrlInfo {
            final_url: server.url(),
            file_size: mock_body.len(),
            file_name: "test_file.bin".to_string(),
            supports_range: false,
            etag: None,
            last_modified: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(file, client, url_info).unwrap();

        let progress_events: Vec<_> = rx.iter().collect();
        handle.join().unwrap();

        let mut file_content = Vec::new();
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut file_content)
            .unwrap();
        assert_eq!(file_content, mock_body);

        assert_eq!(progress_events.len(), 1);
        assert_eq!(progress_events[0].start, 0);
        assert_eq!(progress_events[0].end, mock_body.len() - 1);
        mock.assert();
    }

    #[test]
    fn test_downloads_empty_file_correctly() {
        // 测试空文件下载
        let mock_body = b"";
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(mock_body)
            .create();

        let url_info = UrlInfo {
            final_url: server.url(),
            file_size: mock_body.len(),
            file_name: "empty.bin".to_string(),
            supports_range: false,
            etag: None,
            last_modified: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(file, client, url_info).unwrap();

        let progress_events: Vec<_> = rx.iter().collect();
        handle.join().unwrap();

        // 验证空文件
        let mut file_content = Vec::new();
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut file_content)
            .unwrap();
        assert!(file_content.is_empty());

        // 验证无进度事件
        assert!(progress_events.is_empty());
        mock.assert();
    }

    #[test]
    fn test_downloads_large_file_correctly() {
        let mock_body = [b'a'; 5000];
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(&mock_body)
            .create();

        let url_info = UrlInfo {
            final_url: server.url(),
            file_size: mock_body.len(),
            file_name: "large.bin".to_string(),
            supports_range: false,
            etag: None,
            last_modified: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(file, client, url_info).unwrap();

        let progress_events: Vec<_> = rx.iter().collect();
        handle.join().unwrap();

        // 验证文件内容
        let mut file_content = Vec::new();
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut file_content)
            .unwrap();
        assert_eq!(file_content, mock_body);

        // 验证进度事件总和
        let total_downloaded = progress_events.size();
        assert_eq!(total_downloaded, mock_body.len());
        assert_eq!(progress_events[0].start, 0);
        assert_eq!(progress_events.last().unwrap().end, mock_body.len() - 1);
        mock.assert();
    }

    #[test]
    fn test_downloads_exact_buffer_size_file() {
        let mock_body = vec![b'x'; 4096];
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(&mock_body)
            .create();

        let url_info = UrlInfo {
            final_url: server.url(),
            file_size: mock_body.len(),
            file_name: "exact_buffer.bin".to_string(),
            supports_range: false,
            etag: None,
            last_modified: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(file, client, url_info).unwrap();

        let progress_events: Vec<_> = rx.iter().collect();
        handle.join().unwrap();

        // 验证文件内容
        let mut file_content = Vec::new();
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut file_content)
            .unwrap();
        assert_eq!(file_content, mock_body);

        // 验证进度事件完整性
        let total_downloaded = progress_events.size();
        assert_eq!(total_downloaded, mock_body.len());
        assert_eq!(progress_events[0].start, 0);
        assert_eq!(progress_events.last().unwrap().end, 4095);
        mock.assert();
    }
}
