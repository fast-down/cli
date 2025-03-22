use crate::download_progress::DownloadProgress;

pub trait ProgressesRemain {
    fn get_remain(&self, remain: usize) -> Vec<DownloadProgress>;
}

impl ProgressesRemain for Vec<DownloadProgress> {
    fn get_remain(&self, remain: usize) -> Vec<DownloadProgress> {
        let mut res = Vec::new();
        let mut sum = 0;
        // 从后向前遍历区间
        for c in self.iter().rev() {
            let size = c.size();
            if sum + size <= remain {
                // 如果当前区间完全在 remain 范围内，加入结果
                res.push(c.clone());
                sum += size;
            } else {
                // 如果当前区间跨越了 remain 的边界，分割区间
                let needed = remain - sum;
                if needed > 0 {
                    res.push(DownloadProgress {
                        start: c.end - needed + 1,
                        end: c.end,
                    });
                }
                break;
            }
        }
        // 反转结果，保持原始顺序
        res.reverse();
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_remain() {
        let progresses = vec![
            DownloadProgress { start: 0, end: 4 },   // size = 5
            DownloadProgress { start: 5, end: 9 },   // size = 5
            DownloadProgress { start: 10, end: 14 }, // size = 5
        ];

        // 获取剩余的 5 个字节
        let res = progresses.get_remain(5);
        assert_eq!(
            res,
            vec![
                DownloadProgress { start: 10, end: 14 }, // 最后一个区间
            ]
        );

        // 获取剩余的 7 个字节
        let res = progresses.get_remain(7);
        assert_eq!(
            res,
            vec![
                DownloadProgress { start: 8, end: 9 },   // 第二个区间的部分
                DownloadProgress { start: 10, end: 14 }, // 最后一个区间
            ]
        );

        // 获取剩余的 15 个字节
        let res = progresses.get_remain(15);
        assert_eq!(
            res,
            vec![
                DownloadProgress { start: 0, end: 4 },   // 第一个区间
                DownloadProgress { start: 5, end: 9 },   // 第二个区间
                DownloadProgress { start: 10, end: 14 }, // 第三个区间
            ]
        );

        // 获取剩余的 0 个字节
        let res = progresses.get_remain(0);
        assert_eq!(res, vec![]);

        // 获取剩余的 16 个字节
        let res = progresses.get_remain(16);
        assert_eq!(
            res,
            vec![
                DownloadProgress { start: 0, end: 4 },   // 第一个区间
                DownloadProgress { start: 5, end: 9 },   // 第二个区间
                DownloadProgress { start: 10, end: 14 }, // 第三个区间
            ]
        );

        // 获取剩余的 18 个字节
        let res = progresses.get_remain(18);
        assert_eq!(
            res,
            vec![
                DownloadProgress { start: 0, end: 4 },   // 第一个区间
                DownloadProgress { start: 5, end: 9 },   // 第二个区间
                DownloadProgress { start: 10, end: 14 }, // 第三个区间
            ]
        );
    }
}
