use log::{
    Level,
    LevelFilter,
    Log,
    Metadata,
    Record,
};
use std::io::{self, Stderr, Write};
use termion::{color, style};

pub fn init() {
    struct Logger {
        stream: Stderr,
        pretty: bool,
    }

    impl Log for Logger {
        fn enabled(&self, _: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            let mut stream = self.stream.lock();

            if self.pretty {
                write!(stream, "{}", style::Bold).ok();

                match record.level() {
                    Level::Error => write!(stream, "{}", color::Fg(color::Red)),
                    Level::Warn => write!(stream, "{}", color::Fg(color::Magenta)),
                    Level::Info => write!(stream, "{}", color::Fg(color::Yellow)),
                    Level::Debug => write!(stream, "{}", color::Fg(color::Cyan)),
                    Level::Trace => write!(stream, "{}", color::Fg(color::Blue)),
                }.ok();
            }

            write!(stream, "{}", match record.level() {
                Level::Error => "error",
                Level::Warn => "warn",
                Level::Info => "info",
                Level::Debug => "debug",
                Level::Trace => "trace",
            }).unwrap();

            if self.pretty {
                write!(stream, "{}", style::Reset).ok();
            }

            writeln!(stream, ": {}", record.args()).unwrap();
        }

        fn flush(&self) {}
    }

    let stderr = io::stderr();
    let pretty = termion::is_tty(&stderr);
    let logger = Logger {
        stream: stderr,
        pretty,
    };

    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(LevelFilter::Warn);

    if pretty {
        log::debug!("tty detected, pretty logging is enabled");
    } else {
        log::debug!("stderr is not a tty, pretty logging is disabled");
    }
}

pub fn verbose(verbosity: usize) {
    log::set_max_level(match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    });
}

pub fn quiet() {
    log::set_max_level(LevelFilter::Off);
}
