//! One nonblocking worker for all control clients. Only this module knows
//! Unix sockets; protocol data is also usable by a client.

use super::{protocol::*, state::permitted};
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
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_CLIENTS: usize = 16;
const CHOICES_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct Server {
    messages: mpsc::Sender<Message>,
    wake: UnixStream,
    thread: Option<JoinHandle<()>>,
    path: PathBuf,
}

enum Message {
    Event(Event),
    Stop,
}

impl Server {
    pub fn start(
        dir: &Path,
        state: State,
        commands: async_channel::Sender<Command>,
    ) -> io::Result<Self> {
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
            let thread = thread::Builder::new()
                .name("opencraft-control".into())
                .spawn(move || {
                    Worker {
                        listener,
                        wake: reader,
                        messages: receiver,
                        state,
                        commands,
                        clients: Vec::new(),
                    }
                    .run();
                })?;
            Ok(Self {
                messages,
                wake,
                thread: Some(thread),
                path: path.clone(),
            })
        })();
        if prepared.is_err() {
            let _ = std::fs::remove_file(path);
        }
        prepared
    }

    fn post(&self, message: Message) {
        let _ = self.messages.send(message);
        // A full pipe is already a wake. Never wait on the worker.
        let _ = (&self.wake).write(&[1]);
    }

    pub fn observe(&self, event: Event) {
        self.post(Message::Event(event));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.post(Message::Stop);
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
}

struct Pending {
    engine_id: u64,
    request_id: u64,
    until: Instant,
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
        }
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
    state: State,
    commands: async_channel::Sender<Command>,
    clients: Vec<Client>,
}

impl Worker {
    fn run(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        loop {
            // Drain the pipe BEFORE the queue. An event posted after this
            // drain either appears in the queue or leaves a wake for poll.
            let mut buffer = [0u8; 1024];
            while self.wake.read(&mut buffer).is_ok_and(|n| n > 0) {}
            let mut count = 0;
            for message in self.messages.try_iter().take(256) {
                count += 1;
                match message {
                    Message::Stop => return,
                    Message::Event(event) => {
                        self.state.apply(&event);
                        if let Event::Choices { id, options } = event {
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
                    }
                }
            }
            // A bounded drain preserves fairness. A nonempty queue must
            // not lose its wake when more than one batch was posted.
            let more_events = count == 256;
            for _ in 0..MAX_CLIENTS {
                match self.listener.accept() {
                    Ok((socket, _)) => {
                        if self.clients.len() < MAX_CLIENTS
                            && same_user(&socket)
                            && socket.set_nonblocking(true).is_ok()
                        {
                            self.clients.push(Client::new(socket));
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => return,
                }
            }
            self.clients.retain_mut(|c| c.io().unwrap_or(false));
            let state = &self.state;
            let commands = &self.commands;
            for client in &mut self.clients {
                if client
                    .pending
                    .as_ref()
                    .is_some_and(|p| Instant::now() >= p.until)
                {
                    let pending = client.pending.take().unwrap();
                    client.respond(
                        Response::error(
                            Some(pending.request_id),
                            ErrorCode::Timeout,
                            "The field choices did not arrive in time.",
                        ),
                        &home,
                    );
                }
                if client.ready() {
                    handle(client, state, commands, &home);
                }
            }
            let mut polls = vec![
                libc::pollfd {
                    fd: self.listener.as_raw_fd(),
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
                let events = if !client.output.is_empty() {
                    libc::POLLOUT
                } else if client.pending.is_none() {
                    libc::POLLIN
                } else {
                    0
                };
                polls.push(libc::pollfd {
                    fd: client.socket.as_raw_fd(),
                    events,
                    revents: 0,
                });
            }
            let timeout = if more_events || self.clients.iter().any(Client::ready) {
                0
            } else {
                self.clients
                    .iter()
                    .filter_map(|c| c.pending.as_ref())
                    .map(|p| {
                        p.until
                            .saturating_duration_since(Instant::now())
                            .as_millis()
                            .min(i32::MAX as u128) as i32
                    })
                    .min()
                    .unwrap_or(-1)
            };
            // SAFETY: poll gets a valid, writable array for this call only.
            if unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, timeout) } < 0
                && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
            {
                return;
            }
            let mut index = 2;
            self.clients.retain(|_| {
                let failed =
                    polls[index].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0;
                index += 1;
                !failed
            });
        }
    }
}

fn handle(
    client: &mut Client,
    state: &State,
    commands: &async_channel::Sender<Command>,
    home: &str,
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
            let known = raw.get("method").and_then(|m| m.as_str()).is_some_and(|m| {
                matches!(
                    m,
                    "hello"
                        | "state"
                        | "layout"
                        | "kinds"
                        | "choices"
                        | "profiles"
                        | "pages"
                        | "folders"
                        | "content"
                        | "device_settings"
                        | "history"
                        | "connection"
                )
            });
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
                    "The client and OpenCraft use different protocol versions.",
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
                "Send hello before a read request.",
            ),
            home,
        );
        return;
    }
    let response = match request.method {
        Method::Hello(_) => unreachable!(),
        Method::State(_) => Response::success(id, state),
        Method::Layout(_) => Response::success(id, &state.layout),
        Method::Kinds(_) => Response::success(id, &state.kinds),
        Method::Profiles(_) => Response::success(id, &state.profiles),
        Method::Pages(_) => Response::success(id, &state.pages),
        Method::Folders(_) => Response::success(id, &state.pages.folders),
        Method::Content(_) => Response::success(id, &state.content),
        Method::DeviceSettings(_) => Response::success(id, &state.device_settings),
        Method::History(_) => Response::success(id, &state.history),
        Method::Connection(_) => Response::success(id, &state.connection),
        Method::Choices(params) => {
            let field = state
                .kinds
                .iter()
                .find(|k| k.kind == params.action.kind)
                .and_then(|k| k.fields.iter().find(|f| f.key == params.field));
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
                    // GUI ids count up from 1. Control ids count down and
                    // survive disable/enable, so a late reply cannot match
                    // a new connection after the socket restarts.
                    static NEXT: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(u64::MAX);
                    let engine_id = NEXT.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    let command = Command::Choices {
                        id: engine_id,
                        action,
                        field: params.field,
                    };
                    debug_assert!(permitted(&command));
                    if commands.try_send(command).is_err() {
                        Response::error(
                            Some(id),
                            ErrorCode::Unavailable,
                            "The engine is not available.",
                        )
                    } else {
                        client.pending = Some(Pending {
                            engine_id,
                            request_id: id,
                            until: Instant::now() + CHOICES_TIMEOUT,
                        });
                        return;
                    }
                }
                _ => Response::error(
                    Some(id),
                    ErrorCode::InvalidParams,
                    "The field has no choices.",
                ),
            }
        }
    };
    client.respond(response, home);
}

/// Redact the current home path even in nested actions and handler text.
/// No device serial number enters this module: connection data is only
/// a boolean and the firmware version, never device identity or logs.
fn hide_home(value: &mut serde_json::Value, home: &str) {
    match value {
        serde_json::Value::String(text) if !home.is_empty() && home != "/" => {
            *text = text.replace(home.trim_end_matches('/'), "~")
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| hide_home(v, home)),
        serde_json::Value::Object(items) => {
            let original = std::mem::take(items);
            for (key, mut value) in original {
                let mut key = serde_json::Value::String(key);
                hide_home(&mut key, home);
                hide_home(&mut value, home);
                items.insert(key.as_str().unwrap().into(), value);
            }
        }
        _ => {}
    }
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
        let (send, _) = async_channel::unbounded();
        let server = Server::start(dir.path(), State::default(), send).unwrap();
        assert!(dir.path().join(FILE_NAME).exists());
        drop(server);
        assert!(!dir.path().join(FILE_NAME).exists());
    }
}
