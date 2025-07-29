use super::DownloadResult;
use crate::{ConnectErrorKind, Event, SeqWriter};
use reqwest::{Client, IntoUrl};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub retry_gap: Duration,
    pub write_channel_size: usize,
}

pub async fn download(
    client: Client,
    url: impl IntoUrl,
    mut writer: impl SeqWriter + 'static,
    options: DownloadOptions,
) -> Result<DownloadResult, reqwest::Error> {
    let url = url.into_url()?;
    let (tx, event_chain) = async_channel::unbounded();
    let (tx_write, rx_write) = async_channel::bounded(options.write_channel_size);
    let tx_clone = tx.clone();
    let handle = tokio::spawn(async move {
        while let Ok((spin, data)) = rx_write.recv().await {
            loop {
                match writer.write_sequentially(&data).await {
                    Ok(_) => break,
                    Err(e) => tx_clone.send(Event::WriteError(e)).await.unwrap(),
                }
                tokio::time::sleep(options.retry_gap).await;
            }
            tx_clone.send(Event::WriteProgress(spin)).await.unwrap();
        }
        loop {
            match writer.flush().await {
                Ok(_) => break,
                Err(e) => tx_clone.send(Event::WriteError(e)).await.unwrap(),
            };
            tokio::time::sleep(options.retry_gap).await;
        }
    });
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    tokio::spawn(async move {
        let mut downloaded: u64 = 0;
        let mut response = loop {
            if !running.load(Ordering::Relaxed) {
                tx.send(Event::Abort(0)).await.unwrap();
                return;
            }
            tx.send(Event::Connecting(0)).await.unwrap();
            match client.get(url.clone()).send().await {
                Ok(response) => break response,
                Err(e) => tx
                    .send(Event::ConnectError(0, ConnectErrorKind::Reqwest(e)))
                    .await
                    .unwrap(),
            }
            tokio::time::sleep(options.retry_gap).await;
        };
        tx.send(Event::Downloading(0)).await.unwrap();
        loop {
            let chunk = loop {
                if !running.load(Ordering::Relaxed) {
                    tx.send(Event::Abort(0)).await.unwrap();
                    return;
                }
                match response.chunk().await {
                    Ok(chunk) => break chunk,
                    Err(e) => tx.send(Event::DownloadError(0, e)).await.unwrap(),
                }
                tokio::time::sleep(options.retry_gap).await;
            };
            if chunk.is_none() {
                break;
            }
            let chunk = chunk.unwrap();
            let len = chunk.len() as u64;
            let span = downloaded..(downloaded + len);
            tx.send(Event::DownloadProgress(0, span.clone()))
                .await
                .unwrap();
            tx_write.send((span, chunk)).await.unwrap();
            downloaded += len as u64;
        }
        tx.send(Event::Finished(0)).await.unwrap();
    });
    Ok(DownloadResult::new(event_chain, handle, running_clone))
}

#[cfg(test)]
#[cfg(feature = "file")]
mod tests {
    use super::*;
    use crate::Total;
    use crate::writer::file::SeqFileWriter;
    use tempfile::NamedTempFile;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    fn build_mock_data(size: usize) -> Vec<u8> {
        (0..size).map(|i| (i % 256) as u8).collect()
    }

    #[tokio::test]
    async fn test_downloads_small_file_correctly() {
        // 测试 9B 小文件
        let mock_body = b"test data";
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/small")
            .with_status(200)
            .with_body(mock_body)
            .create_async()
            .await;

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap().into();

        let client = Client::new();
        let result = download(
            client,
            format!("{}/small", server.url()),
            SeqFileWriter::new(file, 8 * 1024 * 1024),
            DownloadOptions {
                retry_gap: Duration::from_secs(1),
                write_channel_size: 1024,
            },
        )
        .await
        .unwrap();

        let mut progress_events = Vec::new();
        while let Ok(event) = result.event_chain.recv().await {
            progress_events.push(event);
        }
        dbg!(&progress_events);
        result.join().await.unwrap();

        let mut file_content = Vec::new();
        File::open(temp_file.path())
            .await
            .unwrap()
            .read_to_end(&mut file_content)
            .await
            .unwrap();
        assert_eq!(file_content, mock_body);

        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::DownloadProgress(_, p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::WriteProgress(p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_downloads_empty_file_correctly() {
        // 测试空文件下载
        let mock_body = b"";
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/empty")
            .with_status(200)
            .with_body(mock_body)
            .create_async()
            .await;

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap().into();

        let client = Client::new();
        let result = download(
            client,
            format!("{}/empty", server.url()),
            SeqFileWriter::new(file, 8 * 1024 * 1024),
            DownloadOptions {
                retry_gap: Duration::from_secs(1),
                write_channel_size: 1024,
            },
        )
        .await
        .unwrap();

        let mut progress_events = Vec::new();
        while let Ok(event) = result.event_chain.recv().await {
            progress_events.push(event);
        }
        dbg!(&progress_events);
        result.join().await.unwrap();

        // 验证空文件
        let mut file_content = Vec::new();
        File::open(temp_file.path())
            .await
            .unwrap()
            .read_to_end(&mut file_content)
            .await
            .unwrap();
        assert!(file_content.is_empty());

        // 验证无进度事件
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::DownloadProgress(_, p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::WriteProgress(p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_downloads_large_file_correctly() {
        let mock_body = build_mock_data(5000);
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/large")
            .with_status(200)
            .with_body(&mock_body)
            .create_async()
            .await;

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap().into();

        let client = Client::new();
        let result = download(
            client,
            format!("{}/large", server.url()),
            SeqFileWriter::new(file, 8 * 1024 * 1024),
            DownloadOptions {
                retry_gap: Duration::from_secs(1),
                write_channel_size: 1024,
            },
        )
        .await
        .unwrap();

        let mut progress_events = Vec::new();
        while let Ok(event) = result.event_chain.recv().await {
            progress_events.push(event);
        }
        dbg!(&progress_events);
        result.join().await.unwrap();

        // 验证文件内容
        let mut file_content = Vec::new();
        File::open(temp_file.path())
            .await
            .unwrap()
            .read_to_end(&mut file_content)
            .await
            .unwrap();
        assert_eq!(file_content, mock_body);

        // 验证进度事件总和
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::DownloadProgress(_, p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::WriteProgress(p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_downloads_exact_buffer_size_file() {
        let mock_body = build_mock_data(4096);
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/exact_buffer_size_file")
            .with_status(200)
            .with_body(&mock_body)
            .create_async()
            .await;

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap().into();

        let client = Client::new();
        let result = download(
            client,
            format!("{}/exact_buffer_size_file", server.url()),
            SeqFileWriter::new(file, 8 * 1024 * 1024),
            DownloadOptions {
                retry_gap: Duration::from_secs(1),
                write_channel_size: 1024,
            },
        )
        .await
        .unwrap();

        let mut progress_events = Vec::new();
        while let Ok(event) = result.event_chain.recv().await {
            progress_events.push(event);
        }
        dbg!(&progress_events);
        result.join().await.unwrap();

        // 验证文件内容
        let mut file_content = Vec::new();
        File::open(temp_file.path())
            .await
            .unwrap()
            .read_to_end(&mut file_content)
            .await
            .unwrap();
        assert_eq!(file_content, mock_body);

        // 验证进度事件完整性
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::DownloadProgress(_, p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        assert_eq!(
            progress_events
                .iter()
                .map(|m| if let Event::WriteProgress(p) = m {
                    p.total()
                } else {
                    0
                })
                .sum::<u64>(),
            mock_body.len() as u64
        );
        mock.assert_async().await;
    }
}
