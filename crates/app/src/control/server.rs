//! One nonblocking worker for all control clients. Only this module knows
//! Unix sockets; protocol data is also usable by a client.

use super::{
    changes::{self, Made},
    protocol::*,
    state::permitted,
    Feed, Port,
};
use opencraft_engine::{Command, Event};
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    os::{
        fd::AsRawFd,
        unix::{
            fs::PermissionsExt,
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_CLIENTS: usize = 16;
/// The limit of a read that waits for a handler. A change has no limit: the
/// engine obeys it also after a long wait, and an error for it would invite
/// a client to send the request again.
const PENDING_TIMEOUT: Duration = Duration::from_secs(5);
/// The time that the worker waits after an error of `accept` that can go
/// away, for example no free file descriptor.
const ACCEPT_PAUSE: Duration = Duration::from_millis(250);

/// The time limits of a connection. A test makes them short.
#[derive(Clone, Copy)]
pub(super) struct Limits {
    /// A connection must send `hello` in this time.
    pub hello: Duration,
    /// A client must read a response in this time.
    pub write: Duration,
    /// A read that waits for a handler or for the engine has this limit. A
    /// change has none.
    pub pending: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            hello: Duration::from_secs(10),
            write: Duration::from_secs(10),
            pending: PENDING_TIMEOUT,
        }
    }
}

pub(super) struct Server {
    waker: Waker,
    feed: Feed,
    thread: Option<JoinHandle<()>>,
    path: PathBuf,
}

pub(super) enum Message {
    /// An answer that a client waits for.
    Event(Event),
    /// The result of one command of a client, from the engine thread. A
    /// success has what the request made.
    Reply {
        id: u64,
        result: Result<Changed, String>,
    },
    Stop,
}

/// A way to put a message in the queue of the worker and wake it. The engine
/// thread uses it. It never waits for the worker.
#[derive(Clone)]
pub(super) struct Waker {
    messages: mpsc::Sender<Message>,
    wake: Arc<UnixStream>,
}

impl Waker {
    pub(super) fn post(&self, message: Message) {
        let _ = self.messages.send(message);
        // A full pipe is already a wake. Never wait on the worker.
        let _ = (&*self.wake).write(&[1]);
    }
}

/// The result of one command for the worker. If the engine drops it, for
/// example at a stop, the worker gets a problem and not a long wait.
struct Answer {
    id: u64,
    waker: Waker,
    sent: bool,
}

impl Answer {
    fn send(mut self, result: Result<Changed, String>) {
        self.sent = true;
        self.waker.post(Message::Reply {
            id: self.id,
            result,
        });
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        if !self.sent {
            self.waker.post(Message::Reply {
                id: self.id,
                result: Err("The engine is not available.".into()),
            });
        }
    }
}

impl Server {
    pub fn start(dir: &Path, feed: Feed, port: Option<Port>) -> io::Result<Self> {
        Self::start_with(dir, feed, port, Limits::default())
    }

    pub(super) fn start_with(
        dir: &Path,
        feed: Feed,
        port: Option<Port>,
        limits: Limits,
    ) -> io::Result<Self> {
        // A response must never have the home directory. Without it, no
        // socket is made. `home_dir` reads `HOME`, then the user database.
        let home = std::env::home_dir()
            .and_then(|home| home.to_str().map(str::to_string))
            .filter(|home| !home.trim_end_matches('/').is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "OpenCraft cannot find the home directory.",
                )
            })?;
        std::fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let listener = UnixListener::bind(&path)?;
        // No connection is accepted until the mode is correct. The peer
        // check is also mandatory, even with the filesystem protection.
        let prepared = (|| {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            listener.set_nonblocking(true)?;
            let (wake, reader) = UnixStream::pair()?;
            wake.set_nonblocking(true)?;
            reader.set_nonblocking(true)?;
            let (messages, receiver) = mpsc::channel();
            let waker = Waker {
                messages,
                wake: Arc::new(wake),
            };
            let worker = Worker {
                listener,
                wake: reader,
                messages: receiver,
                feed: feed.clone(),
                port,
                waker: waker.clone(),
                clients: Vec::new(),
                home,
                limits,
                accept_pause: None,
                accept_failing: false,
            };
            let socket_path = path.clone();
            let thread = thread::Builder::new()
                .name("opencraft-control".into())
                .spawn(move || {
                    // The file goes away when the worker ends for any
                    // reason, also after a panic. A client then gets "not
                    // found", not a refused connection.
                    let _cleanup = Cleanup(socket_path);
                    let mut worker = worker;
                    if let Err(error) = worker.run() {
                        eprintln!("The control socket stopped: {error}");
                    }
                })?;
            // From now on, the engine thread wakes the worker for each
            // answer. The feed holds the state: nothing is lost.
            feed.lock().waker = Some(waker.clone());
            Ok(Self {
                waker,
                feed,
                thread: Some(thread),
                path: path.clone(),
            })
        })();
        if prepared.is_err() {
            let _ = std::fs::remove_file(path);
        }
        prepared
    }
}

/// Remove the socket file when the worker ends.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl Server {
    /// `false` after the worker ended with an error or a panic. The shell
    /// starts a new server then.
    pub(super) fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }
}

#[cfg(test)]
impl Server {
    /// Put a message in the queue of the worker, as the engine thread does.
    pub(super) fn post(&self, message: Message) {
        self.waker.post(message);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.feed.lock().waker = None;
        self.waker.post(Message::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

struct Client {
    socket: UnixStream,
    input: Vec<u8>,
    output: Vec<u8>,
    written: usize,
    hello: bool,
    close: bool,
    pending: Option<Pending>,
    connected: Instant,
    /// When the output was made. The client has a time limit to read it.
    output_since: Instant,
}

struct Pending {
    /// The identity of the engine answer that the client waits for.
    engine_id: u64,
    request_id: u64,
    /// The time limit. `None` for a change.
    until: Option<Instant>,
    work: Work,
}

enum Work {
    Choices,
    Change,
    Read { target: Target },
}

impl Client {
    fn new(socket: UnixStream) -> Self {
        Self {
            socket,
            input: Vec::new(),
            output: Vec::new(),
            written: 0,
            hello: false,
            close: false,
            pending: None,
            connected: Instant::now(),
            output_since: Instant::now(),
        }
    }

    /// When the worker must close this connection, if nothing happens.
    fn expiry(&self, limits: Limits) -> Option<Instant> {
        let hello = (!self.hello).then(|| self.connected + limits.hello);
        let write = (!self.output.is_empty()).then(|| self.output_since + limits.write);
        hello.into_iter().chain(write).min()
    }

    fn respond(&mut self, response: Response, home: &str) {
        let id = response.id;
        let mut value = serde_json::to_value(response).expect("wire response is JSON data");
        hide_home(&mut value, home);
        self.output = serde_json::to_vec(&value).expect("wire response is JSON data");
        if self.output.len() > MAX_RESPONSE {
            self.output = serde_json::to_vec(&Response::error(
                id,
                ErrorCode::ResponseTooLarge,
                "The response is too large. Read one part of the state.",
            ))
            .unwrap();
        }
        self.output.push(b'\n');
        self.written = 0;
        self.output_since = Instant::now();
    }

    /// At most one bounded read and write in each turn. A client that
    /// neither sends nor receives cannot delay any other client.
    fn io(&mut self) -> io::Result<bool> {
        if !self.output.is_empty() {
            match self.socket.write(&self.output[self.written..]) {
                Ok(0) => return Ok(false),
                Ok(count) => self.written += count,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
            if self.written == self.output.len() {
                self.output.clear();
            }
        }
        if self.close {
            return Ok(!self.output.is_empty());
        }
        if self.pending.is_some() || !self.output.is_empty() || self.input.contains(&b'\n') {
            return Ok(true);
        }
        let mut buffer = [0u8; 8192];
        match self.socket.read(&mut buffer) {
            Ok(0) => Ok(false),
            Ok(count) => {
                self.input.extend_from_slice(&buffer[..count]);
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(true),
            Err(e) => Err(e),
        }
    }

    fn ready(&self) -> bool {
        self.output.is_empty()
            && self.pending.is_none()
            && !self.close
            && (self.input.contains(&b'\n') || self.input.len() > MAX_LINE)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}

struct Worker {
    listener: UnixListener,
    wake: UnixStream,
    messages: mpsc::Receiver<Message>,
    feed: Feed,
    port: Option<Port>,
    waker: Waker,
    clients: Vec<Client>,
    home: String,
    limits: Limits,
    /// No `accept` until this time, after an error that can go away.
    accept_pause: Option<Instant>,
    accept_failing: bool,
}

/// Send `command` to the engine. The result comes back to the worker as
/// `Message::Reply` with the identity that this function gives. The engine
/// thread reads what the request made from the cache, after the events of the
/// command and before it obeys the next command.
fn send_with_answer(port: &Port, waker: &Waker, feed: &Feed, command: Command, made: Made) -> u64 {
    // GUI ids count up from 1. Control ids count down and survive
    // disable/enable, so a late reply cannot match a new connection after the
    // socket restarts.
    let id = next_id();
    let answer = Answer {
        id,
        waker: waker.clone(),
        sent: false,
    };
    let feed = feed.clone();
    port(
        command,
        Some(Box::new(move |result| {
            answer.send(result.map(|()| made_by(&feed, &made)))
        })),
    );
    id
}

/// What a change made, with the identities of the cache.
fn made_by(feed: &Feed, made: &Made) -> Changed {
    match made {
        Made::Nothing => Changed::default(),
        Made::Profile => feed.read(|state| Changed {
            profile: state.profiles.active.clone(),
            ..Changed::default()
        }),
        Made::Page => feed.read(|state| Changed {
            page: Some(state.pages.location.page.clone()),
            ..Changed::default()
        }),
        Made::Folder(id) => Changed {
            folder: Some(id.clone()),
            ..Changed::default()
        },
    }
}

impl Worker {
    fn run(&mut self) -> Result<(), String> {
        let home = self.home.clone();
        loop {
            // Drain the pipe BEFORE the queue. An event posted after this
            // drain either appears in the queue or leaves a wake for poll.
            let mut buffer = [0u8; 1024];
            while self.wake.read(&mut buffer).is_ok_and(|n| n > 0) {}
            let mut count = 0;
            for message in self.messages.try_iter().take(256) {
                count += 1;
                match message {
                    Message::Stop => return Ok(()),
                    Message::Event(Event::Choices { id, options }) => {
                        for client in &mut self.clients {
                            if client.pending.as_ref().is_some_and(|p| p.engine_id == id) {
                                let p = client.pending.take().unwrap();
                                client.respond(
                                    Response::success(
                                        p.request_id,
                                        options.iter().map(Choice::from).collect::<Vec<_>>(),
                                    ),
                                    &home,
                                );
                            }
                        }
                    }
                    Message::Event(Event::ProfileRead { id, profile }) => {
                        for client in &mut self.clients {
                            let Some(p) = client.pending.as_ref().filter(|p| p.engine_id == id)
                            else {
                                continue;
                            };
                            let Work::Read { target } = &p.work else {
                                continue;
                            };
                            let response =
                                match profile.as_ref().map_err(Clone::clone).and_then(|p| {
                                    self.feed.read(|state| changes::content(target, p, state))
                                }) {
                                    Ok(content) => Response::success(p.request_id, content),
                                    Err(text) => Response::error(
                                        Some(p.request_id),
                                        ErrorCode::EngineProblem,
                                        &text,
                                    ),
                                };
                            client.pending = None;
                            client.respond(response, &home);
                        }
                    }
                    Message::Event(_) => {}
                    Message::Reply { id, result } => {
                        for client in &mut self.clients {
                            if client.pending.as_ref().is_some_and(|p| p.engine_id == id) {
                                let request_id = client.pending.take().unwrap().request_id;
                                let response = match result.clone() {
                                    Ok(changed) => Response::success(request_id, changed),
                                    Err(text) => Response::error(
                                        Some(request_id),
                                        ErrorCode::EngineProblem,
                                        &text,
                                    ),
                                };
                                client.respond(response, &home);
                            }
                        }
                    }
                }
            }
            // A bounded drain preserves fairness. A nonempty queue must
            // not lose its wake when more than one batch was posted.
            let more_events = count == 256;
            if self
                .accept_pause
                .is_some_and(|until| Instant::now() >= until)
            {
                self.accept_pause = None;
            }
            for _ in 0..MAX_CLIENTS {
                if self.accept_pause.is_some() {
                    break;
                }
                match self.listener.accept() {
                    Ok((socket, _)) => {
                        self.accept_failing = false;
                        self.admit(socket);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        self.accept_failing = false;
                        break;
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted
                        ) =>
                    {
                        continue
                    }
                    Err(e) => {
                        // For example no free file descriptor. The error can
                        // go away: tell the log once, wait, and try again.
                        if !self.accept_failing {
                            eprintln!("The control socket cannot accept a connection: {e}");
                        }
                        self.accept_failing = true;
                        self.accept_pause = Some(Instant::now() + ACCEPT_PAUSE);
                    }
                }
            }
            self.clients.retain_mut(|c| c.io().unwrap_or(false));
            let now = Instant::now();
            let limits = self.limits;
            self.clients
                .retain(|c| c.expiry(limits).is_none_or(|until| now < until));
            for client in &mut self.clients {
                if client
                    .pending
                    .as_ref()
                    .and_then(|p| p.until)
                    .is_some_and(|until| Instant::now() >= until)
                {
                    let pending = client.pending.take().unwrap();
                    client.respond(
                        Response::error(
                            Some(pending.request_id),
                            ErrorCode::Timeout,
                            "The engine response did not arrive in time.",
                        ),
                        &home,
                    );
                }
                if client.ready() {
                    handle(
                        client,
                        &self.feed,
                        self.port.as_ref(),
                        &self.waker,
                        &home,
                        self.limits.pending,
                    );
                }
            }
            let mut polls = vec![
                libc::pollfd {
                    // A negative descriptor is ignored by poll.
                    fd: if self.accept_pause.is_some() {
                        -1
                    } else {
                        self.listener.as_raw_fd()
                    },
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: self.wake.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            for client in &self.clients {
                // A client that waits for the engine is not polled: a client
                // that closed its write side gives POLLHUP at once on
                // macOS, also with data not read yet, and poll would not
                // wait. Its next read or write finds the end.
                let (fd, events) = if !client.output.is_empty() {
                    (client.socket.as_raw_fd(), libc::POLLOUT)
                } else if client.pending.is_none() {
                    (client.socket.as_raw_fd(), libc::POLLIN)
                } else {
                    (-1, 0)
                };
                polls.push(libc::pollfd {
                    fd,
                    events,
                    revents: 0,
                });
            }
            let timeout = if more_events || self.clients.iter().any(Client::ready) {
                0
            } else {
                // The nearest time at which the worker must act.
                self.clients
                    .iter()
                    .filter_map(|c| c.pending.as_ref().and_then(|p| p.until))
                    .chain(self.clients.iter().filter_map(|c| c.expiry(limits)))
                    .chain(self.accept_pause)
                    .map(|until| {
                        // Round up: a wake just before the time is useless.
                        until
                            .saturating_duration_since(Instant::now())
                            .as_millis()
                            .saturating_add(1)
                            .min(i32::MAX as u128) as i32
                    })
                    .min()
                    .unwrap_or(-1)
            };
            // SAFETY: poll gets a valid, writable array for this call only.
            if unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, timeout) } < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(format!("poll failed: {error}"));
                }
            }
            // POLLHUP is not a failure: a client can close its write side and
            // wait for the responses. `Client::io` finds the end of the data.
            let mut index = 2;
            self.clients.retain(|_| {
                let failed = polls[index].revents & (libc::POLLERR | libc::POLLNVAL) != 0;
                index += 1;
                !failed
            });
        }
    }
}

impl Worker {
    /// Keep a new connection of this user, or close it with the error `busy`.
    fn admit(&mut self, socket: UnixStream) {
        if !same_user(&socket) || socket.set_nonblocking(true).is_err() {
            return;
        }
        if self.clients.len() >= MAX_CLIENTS {
            let mut line = serde_json::to_vec(&Response::error(
                None,
                ErrorCode::Busy,
                "OpenCraft has the largest number of control connections. Close one, and try again.",
            ))
            .expect("wire response is JSON data");
            line.push(b'\n');
            // A small write into an empty buffer. If it fails, the client
            // sees the close only.
            let _ = (&socket).write(&line);
            let _ = socket.shutdown(Shutdown::Both);
            return;
        }
        self.clients.push(Client::new(socket));
    }
}

fn handle(
    client: &mut Client,
    feed: &Feed,
    port: Option<&Port>,
    waker: &Waker,
    home: &str,
    pending: Duration,
) {
    let end = client.input.iter().position(|b| *b == b'\n');
    if end.unwrap_or(client.input.len()) > MAX_LINE {
        client.respond(
            Response::error(
                None,
                ErrorCode::LineTooLong,
                "The request line is too long.",
            ),
            home,
        );
        client.close = true;
        return;
    }
    let Some(end) = end else {
        return;
    };
    let line: Vec<_> = client.input.drain(..=end).collect();
    let raw: serde_json::Value = match serde_json::from_slice(&line) {
        Ok(raw) => raw,
        Err(_) => {
            client.respond(
                Response::error(
                    None,
                    ErrorCode::InvalidRequest,
                    "The request must be one JSON object.",
                ),
                home,
            );
            return;
        }
    };
    let id = raw.get("id").and_then(|id| id.as_u64());
    let request: Request = match serde_json::from_value(raw.clone()) {
        Ok(request) => request,
        Err(_) => {
            let known = raw
                .get("method")
                .and_then(|m| m.as_str())
                .is_some_and(|m| METHODS.contains(&m));
            let (code, text) = if known {
                (
                    ErrorCode::InvalidRequest,
                    "The request id or parameters are not correct.",
                )
            } else {
                (
                    ErrorCode::UnknownMethod,
                    "The control method does not exist.",
                )
            };
            client.respond(Response::error(id, code, text), home);
            return;
        }
    };
    let id = request.id;
    if let Method::Hello(params) = request.method {
        if params.version != VERSION {
            client.respond(
                Response::error(
                    Some(id),
                    ErrorCode::VersionMismatch,
                    &format!(
                        "The client uses protocol version {}. OpenCraft uses version {VERSION}.",
                        params.version
                    ),
                ),
                home,
            );
            client.close = true;
        } else {
            client.hello = true;
            client.respond(
                Response::success(
                    id,
                    HelloResult {
                        version: VERSION,
                        app_version: env!("CARGO_PKG_VERSION").into(),
                    },
                ),
                home,
            );
        }
        return;
    }
    if !client.hello {
        client.respond(
            Response::error(
                Some(id),
                ErrorCode::HelloRequired,
                "Send hello before a request.",
            ),
            home,
        );
        return;
    }
    let unavailable = || {
        Response::error(
            Some(id),
            ErrorCode::Unavailable,
            "The engine is not available.",
        )
    };
    let response = match request.method {
        Method::Hello(_) => unreachable!(),
        // Copy the part under the lock, and make the JSON data after it.
        Method::State(_) => Response::success(id, feed.read(|state| state.clone())),
        Method::Layout(_) => Response::success(id, feed.read(|state| state.layout.clone())),
        Method::Kinds(_) => Response::success(id, feed.read(|state| state.kinds.clone())),
        Method::Profiles(_) => Response::success(id, feed.read(|state| state.profiles.clone())),
        Method::Pages(_) => Response::success(id, feed.read(|state| state.pages.clone())),
        Method::Folders(_) => Response::success(id, feed.read(|state| state.pages.folders.clone())),
        Method::Content(ContentParams { target: None }) => {
            Response::success(id, feed.read(|state| state.content.clone()))
        }
        Method::Content(ContentParams {
            target: Some(target),
        }) => match port {
            None => unavailable(),
            Some(port) => {
                let engine_id = next_id();
                port(
                    Command::ReadProfile {
                        id: engine_id,
                        profile: target.profile.clone(),
                    },
                    None,
                );
                client.pending = Some(Pending {
                    engine_id,
                    request_id: id,
                    until: Some(Instant::now() + pending),
                    work: Work::Read { target },
                });
                return;
            }
        },
        Method::DeviceSettings(_) => {
            Response::success(id, feed.read(|state| state.device_settings.clone()))
        }
        Method::History(_) => Response::success(id, feed.read(|state| state.history.clone())),
        Method::Connection(_) => Response::success(id, feed.read(|state| state.connection.clone())),
        Method::Choices(params) => {
            let field = feed.read(|state| {
                state
                    .kinds
                    .iter()
                    .find(|k| k.kind == params.action.kind)
                    .and_then(|k| k.fields.iter().find(|f| f.key == params.field))
                    .cloned()
            });
            let Some(field) = field else {
                client.respond(
                    Response::error(
                        Some(id),
                        ErrorCode::InvalidParams,
                        "The kind or field does not exist.",
                    ),
                    home,
                );
                return;
            };
            match &field.input {
                Input::Choice { options } => Response::success(id, options),
                Input::RuntimeChoice | Input::RuntimeChoiceList => {
                    let Ok(action) = params.action.try_into() else {
                        client.respond(
                            Response::error(
                                Some(id),
                                ErrorCode::InvalidParams,
                                "The action settings are not correct.",
                            ),
                            home,
                        );
                        return;
                    };
                    let engine_id = next_id();
                    let command = Command::Choices {
                        id: engine_id,
                        action,
                        field: params.field,
                    };
                    debug_assert!(permitted(&command));
                    match port {
                        None => unavailable(),
                        Some(port) => {
                            port(command, None);
                            client.pending = Some(Pending {
                                engine_id,
                                request_id: id,
                                until: Some(Instant::now() + pending),
                                work: Work::Choices,
                            });
                            return;
                        }
                    }
                }
                _ => Response::error(
                    Some(id),
                    ErrorCode::InvalidParams,
                    "The field has no choices.",
                ),
            }
        }
        method => match feed.read(|state| changes::command(method, state)) {
            Err(text) => Response::error(Some(id), ErrorCode::InvalidParams, &text),
            Ok(change) => match port {
                None => unavailable(),
                Some(port) => {
                    let engine_id =
                        send_with_answer(port, waker, feed, change.command, change.made);
                    client.pending = Some(Pending {
                        engine_id,
                        request_id: id,
                        until: None,
                        work: Work::Change,
                    });
                    return;
                }
            },
        },
    };
    client.respond(response, home);
}

/// Redact the home directory of the user: in a text (and in the name of an
/// object) that begins with it, at a path boundary, with the rule of the
/// export of a profile. No device serial number enters this module:
/// connection data is only a boolean and the firmware version, never device
/// identity or logs.
fn hide_home(value: &mut serde_json::Value, home: &str) {
    match value {
        serde_json::Value::String(text) => *text = opencraft_engine::share::hide_home(text, home),
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| hide_home(v, home)),
        serde_json::Value::Object(items) => {
            let original = std::mem::take(items);
            for (key, mut value) in original {
                hide_home(&mut value, home);
                items.insert(opencraft_engine::share::hide_home(&key, home), value);
            }
        }
        _ => {}
    }
}

fn next_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    NEXT.fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
}

fn same_user(socket: &UnixStream) -> bool {
    allowed_uid(peer_uid(socket))
}

fn allowed_uid(peer: io::Result<libc::uid_t>) -> bool {
    // SAFETY: geteuid has no preconditions. A failed peer check is a refusal.
    peer.is_ok_and(|uid| uid == unsafe { libc::geteuid() })
}

#[cfg(target_os = "macos")]
fn peer_uid(socket: &UnixStream) -> io::Result<libc::uid_t> {
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: getpeereid writes the two valid output values.
    if unsafe { libc::getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) } == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn peer_uid(socket: &UnixStream) -> io::Result<libc::uid_t> {
    // SAFETY: ucred is an integer-only C structure; zero is valid.
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&credentials) as libc::socklen_t;
    // SAFETY: the output pointer and size refer to credentials.
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    } == 0
        && length as usize == std::mem::size_of_val(&credentials)
    {
        Ok(credentials.uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn peer_uid(_: &UnixStream) -> io::Result<libc::uid_t> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Cannot check the control peer on this platform.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_credentials_accept_only_this_user_and_fail_closed() {
        let (socket, _) = UnixStream::pair().unwrap();
        assert!(same_user(&socket));
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        assert!(!allowed_uid(Ok(uid.wrapping_add(1))));
        assert!(!allowed_uid(Err(io::Error::other("no peer credentials"))));
    }

    #[test]
    fn a_large_response_gets_a_bounded_error_and_home_keys_are_hidden() {
        let (socket, _) = UnixStream::pair().unwrap();
        let mut client = Client::new(socket);
        client.respond(Response::success(9, "X".repeat(MAX_RESPONSE)), "");
        let response: serde_json::Value = serde_json::from_slice(&client.output).unwrap();
        assert_eq!(response["id"], 9);
        assert_eq!(response["error"]["code"], "response_too_large");
        assert!(client.output.len() < 1024);
        let mut value = serde_json::json!({"/home/example/path": ["/home/example/nested"]});
        hide_home(&mut value, "/home/example");
        assert_eq!(value, serde_json::json!({"~/path": ["~/nested"]}));
    }

    #[test]
    fn dropping_the_server_removes_the_socket() {
        let dir = tempfile::Builder::new()
            .prefix("oc")
            .tempdir_in("/tmp")
            .unwrap();
        let server = Server::start(dir.path(), Feed::default(), None).unwrap();
        assert!(dir.path().join(FILE_NAME).exists());
        drop(server);
        assert!(!dir.path().join(FILE_NAME).exists());
    }
}
