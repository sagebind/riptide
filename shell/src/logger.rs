use log::{Level, LevelFilter, Log, Metadata, Record};
use std::{
    io::IsTerminal,
    net::UdpSocket,
    sync::mpsc::{sync_channel, SyncSender},
    thread,
};

pub fn init(level: LevelFilter) {
    let console_logger: Box<dyn Log> = if std::io::stderr().is_terminal() {
        Box::new(ColorLogger { level })
    } else {
        Box::new(PlainLogger { level })
    };

    // Enable debug port for debug builds.
    if cfg!(debug_assertions) {
        log::set_max_level(LevelFilter::Trace);
        log::set_boxed_logger(Box::new(UdpLogger::new(console_logger))).unwrap();
    } else {
        log::set_max_level(level);
        log::set_boxed_logger(console_logger).unwrap();
    }
}

/// Simple logger that sends everything over an UDP port. When paired with a
/// tool like `socat`, this allows you to read even noisy debug logs without
/// messing up the interactive shell.
struct UdpLogger<L> {
    sender: SyncSender<String>,
    inner: L,
}

impl<L> UdpLogger<L> {
    fn new(inner: L) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let (sender, receiver) = sync_channel::<String>(1024);

        thread::spawn(move || {
            for record in receiver.iter() {
                let _ = socket.send_to(record.as_bytes(), "127.0.0.1:1234");
            }
        });

        Self { sender, inner }
    }
}

impl<L: Log> Log for UdpLogger<L> {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let _ = self
            .sender
            .send(format!("{}: {}\n", record.level(), record.args()));
        self.inner.log(record);
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

/// Simple logger that dumps everything to stderr.
struct PlainLogger {
    level: LevelFilter,
}

impl Log for PlainLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if record.level() <= self.level {
            eprintln!("{}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Simple logger that colorizes log messages and writes them to stderr.
struct ColorLogger {
    level: LevelFilter,
}

impl Log for ColorLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if record.level() <= self.level {
            match record.level() {
                Level::Error => eprintln!("\x1b[1m\x1b[31merror\x1b[0m: {}", record.args()),
                Level::Warn => eprintln!("\x1b[1m\x1b[33mwarn\x1b[0m: {}", record.args()),
                Level::Info => eprintln!("\x1b[1m\x1b[32minfo\x1b[0m: {}", record.args()),
                Level::Debug => eprintln!("\x1b[1m\x1b[36mdebug\x1b[0m: {}", record.args()),
                Level::Trace => eprintln!("\x1b[1m\x1b[37mtrace\x1b[0m: {}", record.args()),
            }
        }
    }

    fn flush(&self) {}
}
