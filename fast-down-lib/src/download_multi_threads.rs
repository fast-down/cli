use crate::{
    display_progress, download_single_thread,
    progress::{ProgresTrait, Progress},
};
use color_eyre::{eyre, Result};
use fast_steal::{spawn::Spawn, task_list::TaskList};
use reqwest::{blocking::Client, header, StatusCode};
extern crate alloc;
use alloc::format;
use alloc::{sync::Arc, vec};
use vec::Vec;
extern crate std;
use alloc::string::String;
use std::{
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    thread::{self, JoinHandle},
};

const CHUNK_SIZE: usize = 8 * 1024;

pub fn download_multi_threads(
    url: String,
    threads: usize,
    file_size: usize,
    file: File,
    client: Client,
) -> Result<(crossbeam_channel::Receiver<Progress>, JoinHandle<()>)> {
    let download_chunk = vec![Progress::new(0, file_size)];
    if download_chunk.is_empty() {
        return Err(eyre::eyre!("Download completed"));
    } else if threads == 1
        && download_chunk.len() == 1
        && download_chunk[0].start == 0
        && download_chunk[0].end == file_size
    {
        return download_single_thread::download_single_thread(url, file, client);
    }
    let (tx, rx) = crossbeam_channel::unbounded();
    let (tx_write, rx_write) = crossbeam_channel::unbounded::<(u64, Vec<u8>)>();
    let handle = thread::spawn(move || {
        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
        for (start, data) in rx_write {
            writer.seek(SeekFrom::Start(start)).unwrap();
            writer.write_all(&data).unwrap();
        }
        writer.flush().unwrap();
    });
    let tasks: Arc<TaskList> = Arc::new(download_chunk.into());
    let tasks_clone = tasks.clone();
    tasks.spawn(
        threads,
        |closure| thread::spawn(move || closure()),
        move |_, task, get_task| loop {
            let mut start = task.start();
            let end = task.end();
            if start >= end {
                if get_task() {
                    continue;
                }
                break;
            }
            let range_str = display_progress::display_progress(&tasks_clone.get_range(start..end));
            let mut response = client
                .get(&url)
                .header(header::RANGE, format!("bytes={}", range_str))
                .send()
                .unwrap();
            if response.status() != StatusCode::PARTIAL_CONTENT {
                panic!(
                    "Error: response code is {}, not 206\n{}",
                    response.status(),
                    range_str
                );
            }
            let mut buffer = [0u8; CHUNK_SIZE];
            loop {
                let end = task.end();
                if start >= end {
                    break;
                }
                let remain = end - start;
                let add = CHUNK_SIZE.min(remain);
                task.fetch_add_start(add);
                let len = response.read(&mut buffer).unwrap();
                tx.send(Progress::new(start, start + len)).unwrap();
                tx_write
                    .send((start as u64, buffer[..len].to_vec()))
                    .unwrap();
                start += len;
            }
        },
    );
    Ok((rx, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Total;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_multi_thread_regular_download() {
        let mock_body = vec![b'a'; 3 * 1024];
        let mock_body_clone = mock_body.clone();
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(206)
            .with_body_from_request(move |request| {
                if !request.has_header("Range") {
                    return mock_body_clone.clone();
                }
                let range = request.header("Range")[0];
                println!("range: {:?}", range);
                let mut parts = range
                    .to_str()
                    .unwrap()
                    .rsplit('=')
                    .next()
                    .unwrap()
                    .splitn(2, '-');
                let start = parts.next().unwrap().parse::<usize>().unwrap();
                let end = parts.next().unwrap().parse::<usize>().unwrap();
                mock_body_clone[start..=end].to_vec()
            })
            .create();

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();

        let client = Client::new();
        let (rx, handle) =
            download_multi_threads(server.url(), 3, mock_body.len(), file, client).unwrap();

        let progress_events: Vec<_> = rx.iter().collect();
        handle.join().unwrap();

        let mut file_content = Vec::new();
        File::open(temp_file.path())
            .unwrap()
            .read_to_end(&mut file_content)
            .unwrap();
        assert_eq!(file_content, mock_body);
        assert_eq!(progress_events.total(), mock_body.len());
        mock.assert();
    }
}
