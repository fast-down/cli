mod worker;
mod common;

use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{BorderType, Borders};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use reqwest::Url;
use fast_down::{DownloadResult, UrlInfo};
use crate::common::ClientOptions;

#[derive(Debug)]
pub enum TaskState {
    Pending,
    Fetch,
    Download(DownloadResult),
    Completed
}

impl Default for TaskState {
    fn default() -> Self { Self::Pending }
}

#[derive(Debug)]
pub enum TaskUrlInfoInner {
    Pending(flume::Receiver<Result<UrlInfo, reqwest::Error>>),
    Ready(Rc<UrlInfo>),
}

#[derive(Debug)]
pub struct TaskUrlInfo(Rc<RefCell<TaskUrlInfoInner>>);

impl TaskUrlInfo {
    fn pending(rx: flume::Receiver<Result<UrlInfo, reqwest::Error>>) -> TaskUrlInfo {
        TaskUrlInfo(Rc::new(RefCell::new(TaskUrlInfoInner::Pending(rx))))
    }
}

#[derive(Debug)]
pub struct DownloadTask {
    pub id: TaskId,
    pub url: Arc<Url>,
    pub data: Rc<RefCell<TaskState>>,
    pub info: TaskUrlInfo
}

impl DownloadTask {
    pub fn pending(id: TaskId, url: Url) -> (flume::Sender<Result<UrlInfo, reqwest::Error>>, DownloadTask) {
        let (tx, rx) = flume::bounded(1);
        (tx, DownloadTask {
            id,
            url: url.into(),
            data: Default::default(),
            info: TaskUrlInfo::pending(rx),
        })
    }
}

pub type TaskId = usize;

pub struct App {
    exit: bool,

    selected: Option<TaskId>,
    next_task_id: AtomicUsize,
    tasks: HashMap<TaskId, DownloadTask>,
    clients: slab::Slab<Client>,

    worker: flume::Sender<worker::Task>,

    // todo
}


enum ClientInner {
    Vacant(ClientOptions),
    Occupied(reqwest::Client),
}

struct Client(UnsafeCell<ClientInner>);

impl Client {
    fn get(&self) -> reqwest::Client {
        let inner = unsafe { &mut * self.0.get() };
        match inner {
            ClientInner::Vacant(options) => {
                let client = reqwest::Client::builder().default_headers(options.headers()).build().unwrap();
                *inner = ClientInner::Occupied(client.clone());
                client
            }
            ClientInner::Occupied(client) => client.clone()
        }
    }

    fn new(options: ClientOptions) -> Client {
        Client(UnsafeCell::new(ClientInner::Vacant(options)))
    }
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = flume::bounded(10);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .unwrap();
            runtime.block_on(worker::main(rx));
        });
        Self {
            selected: None,
            tasks: Default::default(),
            next_task_id: Default::default(),
            clients: Default::default(),
            exit: Default::default(),
            worker: tx,
        }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(frame.area());

        frame.render_widget(
            Block::bordered()
                .title(" Tasks ")
                .border_type(BorderType::Rounded),
            layout[0],
        );
        let statistics = Block::bordered()
            .title(" Statistics ")
            .border_type(BorderType::Rounded);
        let statistics = if let Some(selected) = self.selected {
            statistics
        } else {
            statistics.style(Style::default().bg(Color::DarkGray))
        };
        frame.render_widget(
            statistics,
            layout[1],
        );
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    pub(crate) fn next_task_id(&mut self) -> usize {
        self.next_task_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn download_task(&mut self, client: reqwest::Client, url: Url) {
        let id = self.next_task_id();
        let (tx, task) = DownloadTask::pending(id, url);
        self.worker.send(worker::Task::Fetch(client, task.url.clone(), tx)).unwrap();
        self.tasks.insert(id, task);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            },
            _ => {}
        };
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
