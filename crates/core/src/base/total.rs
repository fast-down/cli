use crate::ProgressEntry;

pub trait Total {
    fn total(&self) -> u64;
}

impl Total for ProgressEntry {
    fn total(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

impl<T: Total> Total for Vec<T> {
    fn total(&self) -> u64 {
        self.iter().map(|r| r.total()).sum()
    }
}

impl<T: Total> Total for [T] {
    fn total(&self) -> u64 {
        self.iter().map(|r| r.total()).sum()
    }
}

impl<T: Total> Total for &T {
    fn total(&self) -> u64 {
        (*self).total()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_total() {
        let range = 10..20;
        assert_eq!(range.total(), 10);
    }

    #[test]
    fn test_vec_total() {
        let ranges = vec![1..5, 10..15, 20..30];
        assert_eq!(ranges.total(), (5 - 1) + (15 - 10) + (30 - 20));
    }

    #[test]
    fn test_slice_total() {
        let ranges = [1..5, 10..15, 20..30];
        assert_eq!(ranges.total(), (5 - 1) + (15 - 10) + (30 - 20));
    }

    #[test]
    fn test_empty_range() {
        let range = 0..0;
        assert_eq!(range.total(), 0);
    }

    #[test]
    fn test_empty_vec() {
        let ranges: Vec<ProgressEntry> = vec![];
        assert_eq!(ranges.total(), 0);
    }
}
