use crate::download_progress::DownloadProgress;

pub trait ProgressesSize {
    fn size(&self) -> usize;
}

impl ProgressesSize for Vec<DownloadProgress> {
    fn size(&self) -> usize {
        self.iter().map(|c| c.size()).sum()
    }
}

impl ProgressesSize for &[DownloadProgress] {
    fn size(&self) -> usize {
        self.iter().map(|c| c.size()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_progresses_size() {
        let progresses = vec![
            DownloadProgress::new(0, 99),    // 大小: 100
            DownloadProgress::new(100, 199), // 大小: 100
            DownloadProgress::new(200, 299), // 大小: 100
        ];

        // 总大小应该是 100 + 100 + 100 = 300
        assert_eq!(progresses.size(), 300);
    }

    #[test]
    fn test_slice_progresses_size() {
        let progresses = vec![
            DownloadProgress::new(0, 99),    // 大小: 100
            DownloadProgress::new(100, 199), // 大小: 100
            DownloadProgress::new(200, 299), // 大小: 100
        ];

        let slice: &[DownloadProgress] = &progresses;

        // 总大小应该是 100 + 100 + 100 = 300
        assert_eq!(slice.size(), 300);
    }

    #[test]
    fn test_empty_progresses_size() {
        let progresses: Vec<DownloadProgress> = vec![];

        // 空向量的大小应该是 0
        assert_eq!(progresses.size(), 0);

        let slice: &[DownloadProgress] = &[];

        // 空切片的大小应该是 0
        assert_eq!(slice.size(), 0);
    }

    #[test]
    fn test_single_progress_size() {
        let progress = DownloadProgress::new(0, 99); // 大小: 100
        let progresses = vec![progress];

        // 单个 DownloadProgress 的大小应该是 100
        assert_eq!(progresses.size(), 100);
    }
}
