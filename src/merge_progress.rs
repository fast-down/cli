use crate::download_progress::DownloadProgress;

pub trait MergeProgress {
    fn merge_progress(&mut self, new: DownloadProgress);
}

impl MergeProgress for Vec<DownloadProgress> {
    fn merge_progress(&mut self, new: DownloadProgress) {
        let i = self.partition_point(|old| old.start < new.start);
        if i == self.len() {
            match self.last_mut() {
                Some(last) if last.end >= new.start - 1 => {
                    last.end = last.end.max(new.end);
                }
                _ => self.push(new),
            }
        } else {
            let u1 = if i == 0 {
                false
            } else {
                self[i - 1].can_merge(&new)
            };
            let u2 = self[i].can_merge(&new);
            if u1 && u2 {
                self[i - 1].end = self[i].end;
                self.remove(i);
            } else if u1 {
                self[i - 1].end = new.end;
            } else if u2 {
                self[i].start = new.start;
            } else {
                self.insert(i, new);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_progress() {
        let mut progresses = vec![];
        progresses.merge_progress(DownloadProgress::new(50, 60));
        assert_eq!(progresses, vec![DownloadProgress::new(50, 60)]);

        progresses.merge_progress(DownloadProgress::new(10, 20));
        assert_eq!(
            progresses,
            vec![DownloadProgress::new(10, 20), DownloadProgress::new(50, 60)]
        );

        progresses.merge_progress(DownloadProgress::new(30, 40));
        assert_eq!(
            progresses,
            vec![
                DownloadProgress::new(10, 20),
                DownloadProgress::new(30, 40),
                DownloadProgress::new(50, 60)
            ]
        );

        progresses.merge_progress(DownloadProgress::new(41, 50));
        assert_eq!(
            progresses,
            vec![DownloadProgress::new(10, 20), DownloadProgress::new(30, 60)]
        );

        progresses.merge_progress(DownloadProgress::new(61, 70));
        assert_eq!(
            progresses,
            vec![DownloadProgress::new(10, 20), DownloadProgress::new(30, 70)]
        );

        progresses.merge_progress(DownloadProgress::new(80, 90));
        assert_eq!(
            progresses,
            vec![
                DownloadProgress::new(10, 20),
                DownloadProgress::new(30, 70),
                DownloadProgress::new(80, 90)
            ]
        );

        progresses.merge_progress(DownloadProgress::new(70, 79));
        assert_eq!(
            progresses,
            vec![DownloadProgress::new(10, 20), DownloadProgress::new(30, 90)]
        );

        progresses.merge_progress(DownloadProgress::new(0, 9));
        assert_eq!(
            progresses,
            vec![DownloadProgress::new(0, 20), DownloadProgress::new(30, 90)]
        );

        progresses.merge_progress(DownloadProgress::new(20, 30));
        assert_eq!(progresses, vec![DownloadProgress::new(0, 90)]);

        progresses.merge_progress(DownloadProgress::new(100, 110));
        assert_eq!(
            progresses,
            vec![
                DownloadProgress::new(0, 90),
                DownloadProgress::new(100, 110)
            ]
        );

        progresses.merge_progress(DownloadProgress::new(91, 99));
        assert_eq!(progresses, vec![DownloadProgress::new(0, 110)])
    }
}
