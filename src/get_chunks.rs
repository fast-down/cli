use crate::{download_progress::DownloadProgress, progresses_size::ProgressesSize};

pub fn get_chunks(
    download_chunk: &Vec<DownloadProgress>,
    threads: usize,
) -> Vec<Vec<DownloadProgress>> {
    if threads == 0 {
        panic!("threads must be greater than 0");
    }
    let total_size: usize = download_chunk.size();
    let chunk_size = total_size / threads;
    let mut remaining_size = total_size % threads;
    let mut chunks = Vec::with_capacity(threads);
    let mut iter = download_chunk.iter();
    let mut prev_end_chunk: Option<DownloadProgress> = None;
    for _ in 0..threads {
        let real_chunk_size = if remaining_size > 0 {
            remaining_size -= 1;
            chunk_size + 1
        } else {
            chunk_size
        };
        let mut inner_chunks = Vec::new();
        let mut inner_size = 0;
        if let Some(prev_chunk) = prev_end_chunk.take() {
            let prev_size = prev_chunk.size();
            if prev_size <= real_chunk_size {
                inner_size += prev_size;
                inner_chunks.push(prev_chunk);
            } else {
                let (p1, p2) = prev_chunk.split_at(prev_chunk.start + real_chunk_size - 1);
                inner_chunks.push(p1);
                prev_end_chunk = Some(p2);
                inner_size += real_chunk_size;
            }
        }
        while inner_size < real_chunk_size {
            if let Some(chunk) = iter.next() {
                let chunk_total_size = chunk.size();
                if inner_size + chunk_total_size <= real_chunk_size {
                    inner_chunks.push(chunk.clone());
                    inner_size += chunk_total_size;
                } else {
                    let remain_size = real_chunk_size - inner_size;
                    let (p1, p2) = chunk.split_at(chunk.start + remain_size - 1);
                    inner_chunks.push(p1);
                    inner_size += remain_size;
                    prev_end_chunk = Some(p2);
                }
            } else {
                break;
            }
        }
        if inner_chunks.len() > 0 {
            chunks.push(inner_chunks);
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_chunks() {
        let download_chunk = vec![
            DownloadProgress::new(1, 5),
            DownloadProgress::new(21, 30),
            DownloadProgress::new(41, 50),
            DownloadProgress::new(61, 75),
        ];
        let threads = 2;
        let chunks = get_chunks(&download_chunk, threads);
        assert_eq!(
            chunks,
            vec![
                vec![
                    DownloadProgress { start: 1, end: 5 },
                    DownloadProgress { start: 21, end: 30 },
                    DownloadProgress { start: 41, end: 45 },
                ],
                vec![
                    DownloadProgress { start: 46, end: 50 },
                    DownloadProgress { start: 61, end: 75 },
                ],
            ]
        )
    }

    #[test]
    fn test_even_split_multiple_chunks() {
        let chunks = vec![DownloadProgress::new(1, 10), DownloadProgress::new(11, 20)];
        let result = get_chunks(&chunks, 2);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 10)],
                vec![DownloadProgress::new(11, 20)],
            ]
        );
    }

    #[test]
    fn test_split_single_chunk_three_threads() {
        let chunk = vec![DownloadProgress::new(1, 30)];
        let result = get_chunks(&chunk, 3);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 10)],
                vec![DownloadProgress::new(11, 20)],
                vec![DownloadProgress::new(21, 30)],
            ]
        );
    }

    #[test]
    fn test_remainder_allocation_single_chunk() {
        let chunk = vec![DownloadProgress::new(1, 5)];
        let result = get_chunks(&chunk, 2);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 3)],
                vec![DownloadProgress::new(4, 5)],
            ]
        );
    }

    #[test]
    fn test_single_thread_returns_full_chunks() {
        let chunks = vec![DownloadProgress::new(1, 5), DownloadProgress::new(6, 10)];
        let result = get_chunks(&chunks, 1);
        assert_eq!(
            result,
            vec![vec![
                DownloadProgress::new(1, 5),
                DownloadProgress::new(6, 10)
            ]]
        );
    }

    #[test]
    fn test_splite_big_file() {
        let chunks = vec![DownloadProgress::new(0, 3512893440 - 1)];
        let result = get_chunks(&chunks, 32);
        println!("{:#?}", result);
        assert_eq!(result.len(), 32);
        let chunk_size: usize = result[0].size();
        for chunk_group in result.iter().skip(1) {
            assert_eq!(chunk_size, chunk_group.size());
        }
        let total_size: usize = result.iter().map(|c| c.size()).sum();
        assert_eq!(total_size, 3512893440);
        assert_eq!(
            result[result.len() - 1][result[result.len() - 1].len() - 1].end,
            3512893440 - 1
        );
    }

    #[test]
    fn test_multiple_splits_across_chunks() {
        let chunks = vec![DownloadProgress::new(1, 10), DownloadProgress::new(21, 35)];
        let result = get_chunks(&chunks, 3);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 9)],
                vec![DownloadProgress::new(10, 10), DownloadProgress::new(21, 27)],
                vec![DownloadProgress::new(28, 35)],
            ]
        );
    }

    #[test]
    fn test_zero_size_chunks() {
        let chunks = vec![];
        let result = get_chunks(&chunks, 2);
        assert_eq!(result, Vec::<Vec<DownloadProgress>>::new());
    }

    #[test]
    fn test_more_threads_than_size() {
        let chunk = vec![DownloadProgress::new(1, 3)];
        let result = get_chunks(&chunk, 5);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 1)], // 1 byte
                vec![DownloadProgress::new(2, 2)], // 1 byte
                vec![DownloadProgress::new(3, 3)], // 1 byte
            ]
        );
    }

    #[test]
    fn test_chunk_spans_multiple_threads() {
        let chunk = vec![DownloadProgress::new(1, 100)];
        let result = get_chunks(&chunk, 4); // 每个线程25 bytes
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 25)],
                vec![DownloadProgress::new(26, 50)],
                vec![DownloadProgress::new(51, 75)],
                vec![DownloadProgress::new(76, 100)],
            ]
        );
    }

    #[test]
    fn test_previous_chunk_carried_over() {
        let chunks = vec![DownloadProgress::new(1, 10), DownloadProgress::new(11, 20)];
        let result = get_chunks(&chunks, 3);
        assert_eq!(
            result,
            vec![
                vec![DownloadProgress::new(1, 7)],
                vec![DownloadProgress::new(8, 10), DownloadProgress::new(11, 14)],
                vec![DownloadProgress::new(15, 20)],
            ]
        );
    }

    #[test]
    fn test_get_chunks_with_zero_chunks() {
        let chunks = vec![];
        let result = get_chunks(&chunks, 4);
        assert_eq!(result, Vec::<Vec<DownloadProgress>>::new());
    }

    #[test]
    #[should_panic = "threads must be greater than 0"]
    fn test_get_chunks_with_zero_thread() {
        let chunks = vec![];
        let result = get_chunks(&chunks, 0);
        assert_eq!(result, Vec::<Vec<DownloadProgress>>::new());
    }
}
