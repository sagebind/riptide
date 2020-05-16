use log::{
    Level,
    LevelFilter,
    Log,
    Metadata,
    Record,
};

pub fn init() {
    if atty::is(atty::Stream::Stderr) {
        log::set_boxed_logger(Box::new(ColorLogger)).unwrap();
    } else {
        log::set_boxed_logger(Box::new(PlainLogger)).unwrap();
    }

    log::set_max_level(LevelFilter::Warn);
}

/// Simple logger that dumps everything to stderr.
struct PlainLogger;

impl Log for PlainLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        eprintln!("{}: {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

/// Simple logger that colorizes log messages and writes them to stderr.
struct ColorLogger;

impl Log for ColorLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        match record.level() {
            Level::Error => eprintln!("\x1b[1m\x1b[31merror\x1b[0m: {}", record.args()),
            Level::Warn => eprintln!("\x1b[1m\x1b[33mwarn\x1b[0m: {}", record.args()),
            Level::Info => eprintln!("\x1b[1m\x1b[32minfo\x1b[0m: {}", record.args()),
            Level::Debug => eprintln!("\x1b[1m\x1b[36mdebug\x1b[0m: {}", record.args()),
            Level::Trace => eprintln!("\x1b[1m\x1b[37mtrace\x1b[0m: {}", record.args()),
        }
    }

    fn flush(&self) {}
}
