use crate::progress::{ProgresTrait as _, Progress};

pub fn display_progress(progress: &[Progress]) -> String {
    progress
        .iter()
        .map(|p| p.fmt())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_progress() {
        let progress = vec![Progress::new(0, 10), Progress::new(20, 30)];
        assert_eq!(display_progress(&progress), "0-9,20-29");
    }

    #[test]
    fn test_display_progress_empty() {
        let progress: Vec<Progress> = vec![];
        assert_eq!(display_progress(&progress), "");
    }

    #[test]
    fn test_display_progress_single() {
        let progress = vec![Progress::new(5, 15)];
        assert_eq!(display_progress(&progress), "5-14");
    }
}
