use crate::progress::{ProgresTrait, Progress};
use color_eyre::eyre::Result;
use reqwest::blocking::Client;
extern crate std;
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    thread::{self, JoinHandle},
};
extern crate alloc;
use alloc::string::String;

pub fn download_single_thread(
    url: String,
    file: File,
    client: Client,
) -> Result<(crossbeam_channel::Receiver<Progress>, JoinHandle<()>)> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let (tx_write, rx_write) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        let mut downloaded = 0;
        let mut response = client.get(url).send().unwrap();
        let mut buffer = [0u8; 8 * 1024];
        loop {
            let len = response.read(&mut buffer).unwrap();
            if len == 0 {
                break;
            }
            tx.send(Progress::new(downloaded, downloaded + len))
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
    use crate::total::Total;
    extern crate alloc;
    extern crate std;
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
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

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(server.url(), file, client).unwrap();

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
        assert_eq!(progress_events[0].end, mock_body.len());
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

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(server.url(), file, client).unwrap();

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

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(server.url(), file, client).unwrap();

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
        let total_downloaded = progress_events.total();
        assert_eq!(total_downloaded, mock_body.len());
        assert_eq!(progress_events[0].start, 0);
        assert_eq!(progress_events.last().unwrap().end, mock_body.len());
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

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) = download_single_thread(server.url(), file, client).unwrap();

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
        let total_downloaded = progress_events.total();
        assert_eq!(total_downloaded, mock_body.len());
        assert_eq!(progress_events[0].start, 0);
        assert_eq!(progress_events.last().unwrap().end, mock_body.len());
        mock.assert();
    }
}
