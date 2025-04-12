use core::ops::Range;
pub type Progress = Range<usize>;

pub trait ProgresTrait {
    fn format(&self) -> String;
    fn new(start: usize, end: usize) -> Self;
    fn can_merge(&self, other: &Self) -> bool;
}

impl ProgresTrait for Progress {
    fn format(&self) -> String {
        format!("{}-{}", self.start, self.end - 1)
    }

    fn new(start: usize, end: usize) -> Self {
        if end <= start {
            panic!("end should <= start");
        }
        Self { start, end }
    }

    fn can_merge(&self, b: &Self) -> bool {
        self.start <= b.end && b.start <= self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let progress = Progress::new(0, 100);
        assert_eq!(progress.start, 0);
        assert_eq!(progress.end, 100);

        let progress = Progress::new(0, 1);
        assert_eq!(progress.start, 0);
        assert_eq!(progress.end, 1);
    }

    #[test]
    #[should_panic = "end should <= start"]
    fn test_new_error() {
        Progress::new(10, 10);
    }

    #[test]
    fn test_can_merge_adjacent() {
        let a = Progress::new(0, 5);
        let b = Progress::new(5, 10);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));
    }

    #[test]
    fn test_can_merge_overlapping() {
        let a = Progress::new(0, 5);
        let b = Progress::new(4, 10);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));
    }

    #[test]
    fn test_cannot_merge_non_adjacent_non_overlapping() {
        let a = Progress::new(0, 5);
        let b = Progress::new(7, 10);
        assert!(!a.can_merge(&b));
        assert!(!b.can_merge(&a));
    }

    #[test]
    fn test_can_merge_same_range() {
        let a = Progress::new(0, 5);
        let b = Progress::new(0, 5);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));
    }

    #[test]
    fn test_can_merge_sub() {
        let a = Progress::new(5, 10);
        let b = Progress::new(0, 20);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));

        let a = Progress::new(5, 10);
        let b = Progress::new(0, 10);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));

        let a = Progress::new(5, 10);
        let b = Progress::new(5, 20);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));
    }

    #[test]
    fn test_cannot_merge_disjoint() {
        let a = Progress::new(0, 5);
        let b = Progress::new(6, 15);
        assert!(!a.can_merge(&b));
        assert!(!b.can_merge(&a));
    }
}
