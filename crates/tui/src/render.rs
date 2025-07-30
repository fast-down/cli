use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

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
    let statistics = Block::bordered()
        .title(" Statistics ")
        .border_type(BorderType::Rounded);
    let statistics = if let Some(_) = app.selected {
        statistics
    } else {
        statistics.style(Style::default().bg(Color::DarkGray))
    };
    frame.render_widget(statistics, layout[1]);
}
