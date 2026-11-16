//! The command-line client `opencraft ctl` (decision 0040, section 4).
//!
//! In this mode the process takes no lock, starts no engine, opens no device,
//! and starts no GUI toolkit. `main` calls [`run`] before any other work, so
//! the messages go to the terminal and not to the log file of decision 0024.
//! The client connects to the socket, sends `hello` and one request, writes
//! the result as JSON to the standard output, and stops. It reuses the types
//! of `control/protocol.rs`: a request that has parameters that are not
//! correct does not leave the client.
//!
//! The list of the commands is in [`commands`].

pub mod commands;
mod parse;
#[cfg(test)]
mod tests;

use crate::control::protocol::{
    ErrorCode, HelloParams, HelloResult, ProtocolError, Request, Response, FILE_NAME, MAX_RESPONSE,
    VERSION,
};
use parse::Parsed;
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, BufReader, IsTerminal, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::Duration,
};

/// The exit statuses of decision 0040, section 4.
pub mod status {
    pub const DONE: i32 = 0;
    pub const REFUSED: i32 = 1;
    pub const ARGUMENTS: i32 = 2;
    pub const NOT_RUNNING: i32 = 3;
    pub const CONTROL_OFF: i32 = 4;
    pub const VERSION: i32 = 5;
}

/// What the client needs from its surroundings. A test gives its own.
pub struct Context {
    /// The directory of `control.sock` and `opencraft.lock`.
    pub config_dir: PathBuf,
    /// True if an OpenCraft process holds the lock in `config_dir`.
    pub running: fn(&Path) -> bool,
    /// Write the result with line breaks and indents.
    pub pretty: bool,
}

/// Run `opencraft ctl` with the arguments after `ctl`. The result is the exit
/// status.
pub fn run(arguments: Vec<std::ffi::OsString>, profiles_dir: Option<PathBuf>) -> i32 {
    let arguments: Result<Vec<String>, _> =
        arguments.into_iter().map(|a| a.into_string()).collect();
    let Ok(arguments) = arguments else {
        eprintln!("An argument is not text.");
        return status::ARGUMENTS;
    };
    let Some(profiles_dir) = profiles_dir else {
        eprintln!("OpenCraft cannot find the profiles directory.");
        return status::NOT_RUNNING;
    };
    let context = Context {
        config_dir: crate::settings::config_dir(&profiles_dir),
        running: |dir| crate::lock::holder(dir).is_some(),
        pretty: io::stdout().is_terminal(),
    };
    execute(
        &context,
        &arguments,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

/// Do the work of [`run`], with its own streams.
pub fn execute(
    context: &Context,
    arguments: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let (command, flags) = match parse::parse(arguments) {
        Ok(Parsed::Help(command)) => {
            let text = match command {
                Some(command) => parse::command_help(command),
                None => parse::help(),
            };
            let _ = out.write_all(text.as_bytes());
            return status::DONE;
        }
        Ok(Parsed::Run { command, flags }) => (command, flags),
        Err(problem) => {
            let _ = writeln!(
                err,
                "{problem}\nFor help, write `opencraft ctl --help` or `opencraft ctl <command> --help`."
            );
            return status::ARGUMENTS;
        }
    };
    // Most commands make their request before the connection: an argument
    // that is not correct must not need a running OpenCraft.
    let early = match command.base {
        None => match parse::params(command, &flags, None).and_then(|p| request(command.method, p))
        {
            Ok(request) => Some(request),
            Err(problem) => {
                let _ = writeln!(
                    err,
                    "{problem}\nSee `opencraft ctl {} --help`.",
                    command.name
                );
                return status::ARGUMENTS;
            }
        },
        Some(_) => None,
    };
    let mut connection = match Connection::open(context) {
        Ok(connection) => connection,
        Err(failure) => return failure.report(err),
    };
    let request = match early {
        Some(request) => request,
        None => {
            let base = command.base.expect("a late command has a base");
            let made = connection
                .call(base, json!({}))
                .map_err(|failure| failure.report(err))
                .and_then(|base| {
                    parse::params(command, &flags, Some(base))
                        .and_then(|p| request(command.method, p))
                        .map_err(|problem| {
                            let _ = writeln!(err, "{problem}");
                            status::ARGUMENTS
                        })
                });
            match made {
                Ok(request) => request,
                Err(status) => return status,
            }
        }
    };
    match connection.send(&request) {
        Ok(result) => {
            let text = if context.pretty {
                serde_json::to_string_pretty(&result)
            } else {
                serde_json::to_string(&result)
            }
            .expect("JSON data has a text");
            let _ = writeln!(out, "{text}");
            status::DONE
        }
        Err(failure) => failure.report(err),
    }
}

/// Make a request of the protocol. The types of the protocol check the
/// parameters: this is the check of the client, before it sends.
fn request(method: &str, params: Value) -> Result<Request, String> {
    serde_json::from_value(json!({"id": 2, "method": method, "params": params}))
        .map_err(|error| format!("The arguments are not correct: {error}."))
}

const CLOSED: &str = "The setting \"Allow control by other programs\" can be off now, or \
                       OpenCraft can have stopped.";

/// Why a call did not give a result.
enum Failure {
    NotRunning(String),
    ControlOff,
    Version(String),
    /// OpenCraft answered with an error.
    Refused(ProtocolError),
    /// The connection failed.
    Connection(String),
}

impl Failure {
    /// Write the problem and give the exit status.
    fn report(self, err: &mut dyn Write) -> i32 {
        let (status, text) = match self {
            Self::NotRunning(text) => (status::NOT_RUNNING, text),
            Self::ControlOff => (
                status::CONTROL_OFF,
                "OpenCraft runs, but it does not allow control by other programs. In the \
                 Settings window of OpenCraft, turn on \"Allow control by other programs\"."
                    .into(),
            ),
            Self::Version(text) => (status::VERSION, text),
            Self::Refused(error) if error.code == ErrorCode::VersionMismatch => {
                (status::VERSION, error.text)
            }
            Self::Refused(error) => (status::REFUSED, error.text),
            Self::Connection(text) => (status::REFUSED, text),
        };
        let _ = writeln!(err, "{text}");
        status
    }
}

struct Connection {
    reader: BufReader<UnixStream>,
}

impl Connection {
    /// Connect and send `hello`.
    fn open(context: &Context) -> Result<Self, Failure> {
        let path = context.config_dir.join(FILE_NAME);
        let stream = match UnixStream::connect(&path) {
            Ok(stream) => stream,
            Err(error) => return Err(not_connected(context, &path, &error)),
        };
        // A change has no limit of time in the server. A write must not
        // wait for ever.
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| Failure::Connection(e.to_string()))?;
        let mut connection = Self {
            reader: BufReader::new(stream),
        };
        let hello = serde_json::from_value(json!({
            "id": 1, "method": "hello", "params": HelloParams { version: VERSION }
        }))
        .expect("hello is a request");
        let result = connection.send(&hello)?;
        let hello: HelloResult = serde_json::from_value(result)
            .map_err(|e| Failure::Connection(format!("OpenCraft gave a wrong answer: {e}.")))?;
        if hello.version != VERSION {
            return Err(Failure::Version(format!(
                "This client uses protocol version {VERSION}. OpenCraft {} uses version {}.",
                hello.app_version, hello.version
            )));
        }
        Ok(connection)
    }

    /// A request with no arguments, for example a read.
    fn call(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        let request = request(method, params).map_err(Failure::Connection)?;
        self.send(&request)
    }

    fn send(&mut self, request: &Request) -> Result<Value, Failure> {
        let mut line = serde_json::to_string(request).expect("a request is JSON data");
        line.push('\n');
        self.reader
            .get_mut()
            .write_all(line.as_bytes())
            .map_err(|e| {
                Failure::Connection(format!("OpenCraft closed the connection ({e}). {CLOSED}"))
            })?;
        let mut answer = String::new();
        // One more than the longest response, to find an answer that is too
        // long.
        let limit = (MAX_RESPONSE + 2) as u64;
        let read = (&mut self.reader).take(limit).read_line(&mut answer);
        match read {
            Ok(0) => Err(Failure::Connection(format!(
                "OpenCraft closed the connection before it answered. {CLOSED}"
            ))),
            Ok(_) if !answer.ends_with('\n') => Err(Failure::Connection(
                "The answer of OpenCraft is too long, or it stopped.".into(),
            )),
            Ok(_) => {
                let response: Response = serde_json::from_str(&answer).map_err(|e| {
                    Failure::Connection(format!("OpenCraft gave a wrong answer: {e}."))
                })?;
                response.into_result().map_err(Failure::Refused)
            }
            // Linux reports a connection that the server closed with unread
            // data as a reset.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe
                ) =>
            {
                Err(Failure::Connection(format!(
                    "OpenCraft closed the connection ({error}). {CLOSED}"
                )))
            }
            Err(error) => Err(Failure::Connection(format!(
                "The connection to OpenCraft failed: {error}."
            ))),
        }
    }
}

/// The socket does not take a connection. Find out why, with the lock file of
/// decision 0028: a process of OpenCraft that runs and no socket file mean
/// that the setting is off.
fn not_connected(context: &Context, path: &Path, error: &io::Error) -> Failure {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => {
            if error.kind() == io::ErrorKind::NotFound && (context.running)(&context.config_dir) {
                Failure::ControlOff
            } else {
                Failure::NotRunning(
                    "OpenCraft does not run. Start OpenCraft, and try again.".into(),
                )
            }
        }
        _ => Failure::Connection(format!(
            "OpenCraft cannot connect to {}: {error}.",
            path.display()
        )),
    }
}
