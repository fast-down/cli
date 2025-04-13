extern crate alloc;
use alloc::format;
use alloc::string::String;
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
        Self { start, end }
    }

    fn can_merge(&self, b: &Self) -> bool {
        self.start == b.end || b.start == self.end
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
    fn test_can_merge() {
        let a = Progress::new(0, 5);
        let b = Progress::new(5, 10);
        assert!(a.can_merge(&b));
        assert!(b.can_merge(&a));
    }

    #[test]
    fn test_format() {
        let progress = Progress::new(0, 100);
        assert_eq!(progress.format(), "0-99");
    }
}
