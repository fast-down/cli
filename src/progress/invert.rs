use fast_pull::ProgressEntry;

pub fn invert(progress: &[ProgressEntry], total_size: u64) -> Vec<ProgressEntry> {
    if progress.is_empty() {
        #[allow(clippy::single_range_in_vec_init)]
        return vec![0..total_size];
    }
    let mut result = Vec::with_capacity(progress.len());
    let mut prev_end = 0;
    for range in progress {
        if range.start > prev_end {
            result.push(prev_end..range.start);
        }
        prev_end = range.end;
    }
    if prev_end < total_size {
        result.push(prev_end..total_size);
    }
    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::single_range_in_vec_init)]
    use super::*;

    #[test]
    fn test_reverse_progress() {
        assert_eq!(invert(&[], 10), [0..10]);
        assert_eq!(invert(&[0..5], 10), [5..10]);
        assert_eq!(invert(&[5..10], 10), [0..5]);
        assert_eq!(invert(&[0..5, 7..10], 10), [5..7]);
        assert_eq!(invert(&[0..3, 5..8], 10), [3..5, 8..10]);
        assert_eq!(invert(&[1..3, 5..8], 10), [0..1, 3..5, 8..10]);
    }
}
