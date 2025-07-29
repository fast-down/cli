mod common;
mod worker;

use crate::common::ClientOptions;
use arboard::Clipboard;
use crossterm::event::{self, Event as TermEvent, KeyCode, KeyEvent, KeyEventKind};
use fast_down::file::DownloadOptions;
use fast_down::{ConnectErrorKind, DownloadResult, Event, UrlInfo, WorkerId};
use ratatui::prelude::*;
use ratatui::widgets::{BorderType, Borders, List, Wrap};
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
use smallvec::SmallVec;
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;

type Failures<T> = VecDeque<T>;

#[derive(Debug, Error)]
pub enum DownloadErrors {
    #[error("[Worker{0}] connect error: {1}")]
    Connect(WorkerId, ConnectErrorKind),
    #[error("[Worker {0}] download error: {1}")]
    Download(WorkerId, reqwest::Error),
    #[error("write error: {0}")]
    Write(#[from] io::Error),
}

#[derive(Debug)]
pub enum TaskState {
    Pending(Failures<reqwest::Error>),
    Request(oneshot::Receiver<Result<DownloadResult, io::Error>>),
    Download(Failures<DownloadErrors>, DownloadResult),
    Completed,
    IoError(io::Error),
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Pending(Default::default())
    }
}

#[derive(Debug)]
pub enum TaskUrlInfo {
    Pending(flume::Receiver<Result<UrlInfo, reqwest::Error>>),
    Ready(Rc<UrlInfo>),
}

impl TaskUrlInfo {
    pub fn unwrap(&self) -> Rc<UrlInfo> {
        match self {
            TaskUrlInfo::Pending(_) => {
                panic!("called TaskUrlInfo::unwrap on a Pending TaskUrlInfo")
            }
            TaskUrlInfo::Ready(rc) => rc.clone(),
        }
    }

    pub unsafe fn unwrap_unchecked(&self) -> Rc<UrlInfo> {
        match self {
            TaskUrlInfo::Pending(_) => unsafe { std::hint::unreachable_unchecked() },
            TaskUrlInfo::Ready(rc) => rc.clone(),
        }
    }
}

impl TaskUrlInfo {
    fn pending(rx: flume::Receiver<Result<UrlInfo, reqwest::Error>>) -> TaskUrlInfo {
        TaskUrlInfo::Pending(rx)
    }
}

#[derive(Debug)]
pub struct DownloadTask {
    pub id: TaskId,
    pub client_id: ClientId,
    pub url: Arc<Url>,
    pub path: Option<PathBuf>,
    /// automatically starts the download after fetch
    pub auto: bool,
    pub(crate) download_options: Option<DownloadOptions>,
    /// `None` -> always retry
    /// `Some(count)` -> retry `count` times
    pub retry: Option<NonZeroUsize>,
    pub state: TaskState,
    pub info: TaskUrlInfo,
}

impl DownloadTask {
    pub(crate) fn state_icon(&self) -> &str {
        match self.state {
            TaskState::Pending(_) => match self.info {
                TaskUrlInfo::Pending(_) => "🔍",
                TaskUrlInfo::Ready(_) => "🫧",
            },
            TaskState::Request(_) => "⏳",
            TaskState::Download(_, _) => "🚚",
            TaskState::Completed => "✅",
            TaskState::IoError(_) => "💥",
        }
    }
}

impl DownloadTask {
    pub fn pending(
        id: TaskId,
        client_id: ClientId,
        path: Option<PathBuf>,
        options: Option<DownloadOptions>,
        url: Url,
    ) -> (flume::Sender<Result<UrlInfo, reqwest::Error>>, DownloadTask) {
        let (tx, rx) = flume::bounded(1);
        (
            tx,
            DownloadTask {
                id,
                path,
                client_id,
                retry: None,
                url: url.into(),
                state: Default::default(),
                info: TaskUrlInfo::pending(rx),
                auto: options.is_some(),
                download_options: options,
            },
        )
    }
}

pub type TaskId = usize;

pub type ClientId = usize;

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
        let inner = unsafe { &mut *self.0.get() };
        match inner {
            ClientInner::Vacant(options) => {
                let client = reqwest::Client::builder()
                    .default_headers(options.headers())
                    .build()
                    .unwrap();
                *inner = ClientInner::Occupied(client.clone());
                client
            }
            ClientInner::Occupied(client) => client.clone(),
        }
    }

    fn new(options: ClientOptions) -> Client {
        Client(UnsafeCell::new(ClientInner::Vacant(options)))
    }
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = flume::bounded(8);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(worker::main(rx));
        });
        let mut clients: slab::Slab<Client> = Default::default();
        clients.insert(Client::new(ClientOptions::builder().build()));
        Self {
            selected: None,
            tasks: Default::default(),
            next_task_id: Default::default(),
            clients,
            exit: Default::default(),
            worker: tx,
        }
    }
}

// const VERSION: &str = env!("CARGO_PKG_VERSION");

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.poll_events();
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(frame.area());

        let tasks_block = Block::bordered()
            .title(" Tasks ")
            .border_type(BorderType::Rounded);

        frame.render_widget(
            List::new(self.tasks.values().map(|task| {
                let style = if self.selected == Some(task.id) {
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
        let statistics = if let Some(selected) = self.selected {
            statistics
        } else {
            statistics.style(Style::default().bg(Color::DarkGray))
        };
        frame.render_widget(statistics, layout[1]);
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn poll_events(&mut self) {
        let mut pending_downloads: SmallVec<[TaskId; 64]> = SmallVec::new();
        let mut pending_completes: SmallVec<[TaskId; 64]> = SmallVec::new();
        for task in self.tasks.values_mut() {
            match &mut task.state {
                TaskState::Pending(failures) => match &task.info {
                    TaskUrlInfo::Pending(receiver) => match receiver.try_recv() {
                        Ok(Ok(value)) => {
                            task.info = TaskUrlInfo::Ready(value.into());
                        }
                        Ok(Err(err)) => failures.push_back(err),
                        Err(flume::TryRecvError::Empty) => {}
                        Err(flume::TryRecvError::Disconnected) => panic!("worker disconnect"),
                    },
                    TaskUrlInfo::Ready(_) if task.auto => {
                        pending_downloads.push(task.id);
                    }
                    TaskUrlInfo::Ready(_) => {}
                },
                TaskState::Request(rx) => match rx.try_recv() {
                    Ok(Ok(result)) => task.state = TaskState::Download(Default::default(), result),
                    Ok(Err(err)) => task.state = TaskState::IoError(err),
                    Err(oneshot::TryRecvError::Empty) => {}
                    Err(oneshot::TryRecvError::Disconnected) => panic!("worker disconnect"),
                },
                TaskState::Download(failures, result) => {
                    loop {
                        match result.event_chain.try_recv() {
                            Ok(ev) => match ev {
                                Event::Connecting(id) => {
                                    // eprintln!("{}: connected", id);
                                }
                                Event::ConnectError(id, err) => {
                                    failures.push_back(DownloadErrors::Connect(id, err))
                                }
                                Event::Downloading(id) => {
                                    // eprintln!("{}: downloading", id);
                                }
                                Event::DownloadError(id, err) => {
                                    failures.push_back(DownloadErrors::Download(id, err))
                                }
                                Event::DownloadProgress(id, progress) => {
                                    // eprintln!("{}: download progress {:?}", id, progress);
                                }
                                Event::WriteError(err) => {
                                    failures.push_back(DownloadErrors::Write(err))
                                }
                                Event::WriteProgress(progress) => {
                                    // eprintln!("write progress {:?}", progress);
                                }
                                Event::Finished(id) => {
                                    // eprintln!("{}: finished", id);
                                }
                                Event::Abort(id) => {
                                    // eprintln!("{}: aborted", id);
                                }
                            },
                            Err(async_channel::TryRecvError::Empty) => break,
                            Err(async_channel::TryRecvError::Closed) => {
                                pending_completes.push(task.id);
                                break;
                            }
                        }
                    }
                }
                TaskState::IoError(_) => {}
                TaskState::Completed => {}
            }
        }
        for task_id in pending_completes {
            match self.tasks.get_mut(&task_id) {
                None => {}
                Some(task) => {
                    task.state = TaskState::Completed;
                }
            }
        }
        for task_id in pending_downloads {
            let mut task = self.tasks.get_mut(&task_id).unwrap();
            let client = self.clients.get(task.client_id).unwrap().get();
            self.worker
                .send(Self::create_download_command(
                    &task.info.unwrap(),
                    client,
                    &mut task,
                    None,
                    None,
                ))
                .unwrap();
        }
    }

    fn create_download_command(
        info: &UrlInfo,
        client: reqwest::Client,
        task: &mut DownloadTask,
        path: Option<PathBuf>,
        options: Option<DownloadOptions>,
    ) -> worker::Task {
        let (tx, rx) = oneshot::channel();
        task.state = TaskState::Request(rx);
        let path = path
            .or_else(|| task.path.take())
            .unwrap_or_else(|| (&info.file_name).into());
        let mut options = options.or_else(|| task.download_options.take()).unwrap();
        if !info.can_fast_download {
            options.concurrent = None;
        }
        worker::Task::Download(
            client,
            task.url.clone(),
            vec![0..info.file_size],
            info.file_size,
            path,
            options,
            tx,
        )
    }

    pub(crate) fn next_task_id(&mut self) -> usize {
        self.next_task_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn create_task(
        &mut self,
        client_id: ClientId,
        path: Option<PathBuf>,
        options: Option<DownloadOptions>,
        url: Url,
    ) {
        let id = self.next_task_id();
        let (tx, task) = DownloadTask::pending(id, client_id, path, options, url);
        self.worker
            .send(worker::Task::Fetch(
                self.clients.get(client_id).unwrap().get(),
                task.url.clone(),
                tx,
            ))
            .unwrap();
        self.tasks.insert(id, task);
    }

    // pub fn download_task(&mut self, ) {
    //     let id = self.next_task_id();
    //     let (tx, task) = DownloadTask::pending(id, url);
    //     self.worker.send(worker::Task::Download(..)).unwrap();
    //     self.tasks.insert(id, task);
    // }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('p') => {
                let mut clipboard = Clipboard::new().unwrap();
                if let Ok(content) = clipboard.get_text() {
                    if let Ok(url) = Url::parse(&content) {
                        self.create_task(
                            /* todo: show options to user to select client */ 0,
                            None,
                            Some(DownloadOptions {
                                concurrent: NonZeroUsize::new(4),
                                write_buffer_size: 1024,
                                retry_gap: Default::default(),
                                write_channel_size: 4,
                            }),
                            url,
                        );
                    } else {
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::poll(Duration::from_millis(50))? {
            true => match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                TermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
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
