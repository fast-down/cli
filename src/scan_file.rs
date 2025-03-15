use crate::download_progress::DownloadProgress;
use color_eyre::eyre::Result;
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;

const CHUNK_SIZE: usize = 1024 * 4; // 4K 对齐会更快

pub fn scan_file(file: &File) -> Result<Vec<DownloadProgress>> {
    // 使用内存映射打开文件
    let mmap = unsafe { MmapOptions::new().map(file)? };
    // 并行处理每个块
    let ranges: Vec<_> = mmap
        .par_chunks(CHUNK_SIZE)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            let mut pos = chunk_idx * CHUNK_SIZE;
            let mut ranges = Vec::new();
            let mut current_start: Option<usize> = None;
            // 扫描当前块中的零字节
            for &byte in chunk.iter() {
                if byte == 0 {
                    if current_start.is_none() {
                        current_start = Some(pos);
                    }
                } else if let Some(start) = current_start.take() {
                    ranges.push(DownloadProgress::new(start, pos - 1));
                }
                pos += 1;
            }
            // 处理块末尾的未闭合区间
            if let Some(start) = current_start {
                ranges.push(DownloadProgress::new(start, pos - 1));
            }
            ranges
        })
        .filter(|r| !r.is_empty())
        .collect();
    Ok(merge_ranges(ranges))
}

// 合并相邻的区间
fn merge_ranges(ranges: Vec<Vec<DownloadProgress>>) -> Vec<DownloadProgress> {
    if ranges.len() < 2 {
        return ranges.iter().flatten().cloned().collect();
    }
    ranges
        .iter()
        .skip(1)
        .fold(ranges[0].clone(), |mut acc, range_group| {
            match acc.last_mut() {
                Some(last) if last.end + 1 == range_group[0].start => {
                    last.end = range_group[0].end;
                    acc.extend(range_group.iter().skip(1).cloned());
                }
                _ => acc.extend(range_group.iter().cloned()),
            }
            acc
        })
}

#[cfg(test)]
mod tests_merge_ranges {
    use super::*;

    #[test]
    fn test_merge_ranges_less_than_2() {
        let ranges = vec![];
        assert_eq!(merge_ranges(ranges), vec![]);
        let ranges = vec![vec![DownloadProgress::new(45, 50)]];
        assert_eq!(merge_ranges(ranges), vec![DownloadProgress::new(45, 50)]);
    }

    #[test]
    fn test_merge_ranges_single_group() {
        let ranges = vec![vec![
            DownloadProgress::new(0, 10),
            DownloadProgress::new(11, 20),
        ]];
        assert_eq!(
            merge_ranges(ranges),
            vec![DownloadProgress::new(0, 10), DownloadProgress::new(11, 20)]
        );
    }

    #[test]
    fn test_merge_ranges_multiple_groups_no_overlap() {
        let ranges = vec![
            vec![DownloadProgress::new(0, 10)],
            vec![DownloadProgress::new(20, 30)],
        ];
        assert_eq!(
            merge_ranges(ranges),
            vec![DownloadProgress::new(0, 10), DownloadProgress::new(20, 30)]
        );
    }

    #[test]
    fn test_merge_ranges_complex() {
        let ranges = vec![
            vec![DownloadProgress::new(0, 5), DownloadProgress::new(10, 15)],
            vec![DownloadProgress::new(16, 20), DownloadProgress::new(25, 30)],
            vec![DownloadProgress::new(35, 40)],
        ];
        assert_eq!(
            merge_ranges(ranges),
            vec![
                DownloadProgress::new(0, 5),
                DownloadProgress::new(10, 20),
                DownloadProgress::new(25, 30),
                DownloadProgress::new(35, 40)
            ]
        );
    }
}

#[cfg(test)]
mod tests_scan_file {
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn test_empty_file() {
        let file = tempfile().unwrap();
        let result = scan_file(&file).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_all_zeros_single_chunk() {
        let mut file = tempfile().unwrap();
        let data = vec![0u8; CHUNK_SIZE];
        file.write_all(&data).unwrap();
        let result = scan_file(&file).unwrap();
        assert_eq!(vec![DownloadProgress::new(0, CHUNK_SIZE - 1)], result);
    }

    #[test]
    fn test_all_zeros_multiple_chunks() {
        let mut file = tempfile().unwrap();
        let data = vec![0u8; CHUNK_SIZE * 3];
        file.write_all(&data).unwrap();
        let result = scan_file(&file).unwrap();
        assert_eq!(vec![DownloadProgress::new(0, CHUNK_SIZE * 3 - 1)], result);
    }

    #[test]
    fn test_cross_chunk_zeros() {
        let mut file = tempfile().unwrap();
        let mut data = vec![1u8; CHUNK_SIZE];
        // 第一个块的最后一个字节为0
        *data.last_mut().unwrap() = 0;
        // 第二个块全部为0
        data.extend(vec![0u8; CHUNK_SIZE]);
        file.write_all(&data).unwrap();

        let result = scan_file(&file).unwrap();
        // 应该合并成一个区间
        assert_eq!(
            vec![DownloadProgress::new(CHUNK_SIZE - 1, 2 * CHUNK_SIZE - 1)],
            result
        );
    }

    #[test]
    fn test_multiple_non_contiguous_ranges() {
        let mut file = tempfile().unwrap();
        let mut data = Vec::new();
        // 块1: 0-99为0，100-199为非零
        data.extend(vec![0u8; 100]);
        data.extend(vec![1u8; CHUNK_SIZE - 100]);
        // 块2: 中间部分为0
        data.extend(vec![0u8; 150]);
        data.extend(vec![1u8; CHUNK_SIZE - 150]);
        // 块3: 末尾为0
        data.extend(vec![0u8; 200]);

        file.write_all(&data).unwrap();

        let result = scan_file(&file).unwrap();
        assert_eq!(
            vec![
                DownloadProgress::new(0, 99),
                DownloadProgress::new(CHUNK_SIZE, CHUNK_SIZE + 149),
                DownloadProgress::new(2 * CHUNK_SIZE, 2 * CHUNK_SIZE + 199),
            ],
            result
        );
    }

    #[test]
    fn test_partial_end_chunk() {
        let mut file = tempfile().unwrap();
        let data = vec![0u8; CHUNK_SIZE + 5]; // 总大小超过一个块
        file.write_all(&data).unwrap();

        let result = scan_file(&file).unwrap();
        assert_eq!(vec![DownloadProgress::new(0, CHUNK_SIZE + 4)], result);
    }

    #[test]
    fn test_non_zero_file() {
        let mut file = tempfile().unwrap();
        let data = vec![1u8; CHUNK_SIZE * 2];
        file.write_all(&data).unwrap();

        let result = scan_file(&file).unwrap();
        assert!(result.is_empty());
    }
}
