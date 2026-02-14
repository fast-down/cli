use crate::fmt;
use crossterm::{QueueableCommand, cursor, style::Print, terminal};
use fast_down::{Merge, ProgressEntry, Total};
use parking_lot::Mutex;
use std::{
    io::{self, Stderr, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

const BLOCK_CHARS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

#[derive(Debug)]
pub struct Painter {
    pub progress: Vec<ProgressEntry>,
    pub width: u16,
    pub start: Instant,
    pub alpha: f64,
    pub file_size: u64,
    pub prev_size: u64,
    pub curr_size: u64,
    pub avg_speed: f64,
    pub repaint_duration: Duration,
    pub last_repaint_time: Instant,
    has_progress: bool,
    stderr: Stderr,
}

impl Painter {
    pub fn new(
        init_progress: Vec<ProgressEntry>,
        file_size: u64,
        progress_width: u16,
        alpha: f64,
        repaint_duration: Duration,
        start: Instant,
    ) -> io::Result<Self> {
        let init_size = init_progress.total();
        let mut stderr = io::stderr();
        stderr.queue(cursor::Hide)?;
        Ok(Self {
            progress: init_progress,
            file_size,
            width: progress_width,
            alpha,
            repaint_duration,
            start,
            prev_size: init_size,
            curr_size: init_size,
            avg_speed: 0.0,
            last_repaint_time: Instant::now(),
            has_progress: false,
            stderr,
        })
    }

    pub fn reset_progress(&mut self) {
        self.progress.clear();
        self.prev_size = 0;
        self.curr_size = 0;
        self.avg_speed = 0.0;
        self.start = Instant::now();
    }

    pub fn start_update_thread(painter_arc: Arc<Mutex<Self>>) -> JoinHandle<()> {
        let duration = {
            let painter = painter_arc.lock();
            assert_ne!(painter.width, 0);
            painter.repaint_duration
        };
        tokio::spawn(async move {
            loop {
                let should_stop = {
                    let mut painter = painter_arc.lock();
                    painter.update().unwrap();
                    painter.file_size > 0 && painter.curr_size >= painter.file_size
                };
                if should_stop {
                    break;
                }
                tokio::time::sleep(duration).await;
            }
        })
    }

    pub fn add(&mut self, p: ProgressEntry) {
        if self.width == 0 {
            return;
        }
        self.progress.merge_progress(p);
        self.curr_size = self.progress.total();
    }

    fn reset_pos(&mut self) -> io::Result<()> {
        if self.has_progress {
            self.stderr
                .queue(cursor::MoveUp(1))?
                .queue(cursor::MoveUp(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    pub fn update(&mut self) -> io::Result<()> {
        if self.width == 0 {
            return Ok(());
        }
        let repaint_elapsed = self.last_repaint_time.elapsed().as_millis();
        self.last_repaint_time = Instant::now();
        let curr_dsize = self.curr_size - self.prev_size;
        self.prev_size = self.curr_size;
        let curr_speed = if repaint_elapsed > 0 {
            (curr_dsize * 1000) as f64 / repaint_elapsed as f64
        } else {
            0.0
        };
        self.avg_speed = self.avg_speed * self.alpha + curr_speed * (1.0 - self.alpha);
        let line1 = if self.file_size == 0 {
            format!(
                "|{}| {:>6.2}% ({:>8}/Unknown)",
                BLOCK_CHARS[0].to_string().repeat(self.width as usize),
                0.0,
                fmt::format_size(self.curr_size as f64),
            )
        } else {
            let get_percent = (self.curr_size as f64 / self.file_size as f64) * 100.0;
            let per_bytes = self.file_size as f64 / self.width as f64;
            let mut bar_values = vec![0u64; self.width as usize];
            let mut index = 0;
            for i in 0..self.width {
                let start_byte = i as f64 * per_bytes;
                let end_byte = (start_byte + per_bytes) as u64;
                let start_byte = start_byte as u64;
                let mut block_total = 0;
                for segment in &self.progress[index..] {
                    if segment.end <= start_byte {
                        index += 1;
                        continue;
                    }
                    if segment.start >= end_byte {
                        break;
                    }
                    let overlap_start = segment.start.max(start_byte);
                    let overlap_end = segment.end.min(end_byte);
                    if overlap_start < overlap_end {
                        block_total += overlap_end - overlap_start;
                    }
                }
                bar_values[i as usize] = block_total;
            }
            let bar_str: String = bar_values
                .iter()
                .map(|&count| {
                    BLOCK_CHARS
                        [((count as f64 / per_bytes * (BLOCK_CHARS.len() - 1) as f64).round()
                            as usize)
                            .min(BLOCK_CHARS.len() - 1)]
                })
                .collect();
            format!(
                "|{}| {:>6.2}% ({:>8}/{})",
                bar_str,
                get_percent,
                fmt::format_size(self.curr_size as f64),
                fmt::format_size(self.file_size as f64),
            )
        };
        let line2 = t!(
            "progress.desc",
            time_spent = fmt::format_time(self.start.elapsed().as_secs()),
            time_left = if self.file_size == 0 { "Unknown".to_string() } else { fmt::format_time(((self.file_size - self.curr_size) as f64 / self.avg_speed) as u64) },
            speed = fmt::format_size(self.avg_speed) : {:>8},
        );
        self.reset_pos()?;
        self.stderr.queue(Print(line1))?;
        self.stderr
            .queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;
        self.stderr.queue(Print("\n"))?;
        self.stderr.queue(Print(&line2))?;
        self.stderr
            .queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;
        self.stderr.queue(Print("\n"))?;
        self.stderr.flush()?;
        self.has_progress = true;
        Ok(())
    }

    pub fn print(&mut self, msg: &str) -> io::Result<()> {
        self.reset_pos()?;
        for line in msg.lines() {
            self.stderr.queue(Print(line))?;
            self.stderr
                .queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;
            self.stderr.queue(Print("\n"))?;
        }
        self.has_progress = false;
        self.update()?;
        Ok(())
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        let _ = self.stderr.queue(cursor::Show);
        let _ = self.stderr.flush();
    }
}
