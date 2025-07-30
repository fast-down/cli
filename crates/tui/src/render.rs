use crate::app::App;
use crate::state::{FDWorkerState, TaskState};
use crate::widget::stats::WorkerStats;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct MainPageState {
    widget_pool: swimmer::Pool<WorkerStats>,
}

impl MainPageState {
    pub fn new() -> MainPageState {
        MainPageState {
            widget_pool: swimmer::Pool::with_size(2),
        }
    }

    pub(crate) fn fetch_stats_widget(&self) -> swimmer::Recycled<WorkerStats> {
        self.widget_pool.get()
    }
}

pub fn init_state(app: &mut App) {
    app.states.insert(MainPageState::new());
}

pub fn draw_main(app: &App, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(frame.area());

    let tasks_block = Block::bordered()
        .title(" Tasks ")
        .border_type(BorderType::Rounded);

    let tasks = app.tasks.values().skip(app.scroll.unwrap_or(0));
    frame.render_widget(
        List::new(tasks.map(|task| {
            let style = if app.selected == Some(task.id) {
                Style::default().fg(Color::White).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Cyan)
            };
            Text::styled(format!("{} {}", task.state_icon(), task.url), style)
        }))
        .block(tasks_block),
        layout[0],
    );

    let statistics_block = Block::bordered()
        .title(" Statistics ")
        .border_type(BorderType::Rounded);
    if let Some(id) = app.selected {
        frame.render_widget(&statistics_block, layout[1]);
        let task = app.tasks.get(&id).unwrap();
        match &task.state {
            TaskState::Pending(_) => { /* todo */ }
            TaskState::Request(_, _) => { /* todo */ }
            TaskState::Download(statistics, _, _) => {
                let mut s_rect = statistics_block.inner(layout[1]);
                frame.render_widget(statistics_block, layout[1]);
                let state = app.states.get::<MainPageState>().unwrap();
                let mut wid = state.fetch_stats_widget();
                for idx in 0..statistics.state.len() {
                    s_rect = wid.render(
                        s_rect,
                        false,
                        Span::raw("[Worker 0]"),
                        &statistics.state[idx],
                        &statistics.download_entries[idx],
                        &[0..756],
                        statistics.written,
                        statistics.downloaded,
                        statistics.total,
                        frame.buffer_mut(),
                    );
                }
            }
            TaskState::Completed => {}
            TaskState::IoError(_) => { /* todo */ }
        }
    } else {
        frame.render_widget(
            statistics_block.style(Style::default().bg(Color::DarkGray)),
            layout[1],
        );
    };
}
