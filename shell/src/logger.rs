use log::{
    Level,
    LevelFilter,
    Log,
    Metadata,
    Record,
};
use std::io::Write;
use termcolor::{
    Color,
    ColorChoice,
    ColorSpec,
    StandardStream,
    WriteColor,
};

pub fn init() {
    struct Logger {
        out: StandardStream,
        pretty: bool,
    }

    impl Log for Logger {
        fn enabled(&self, _: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            let (name, color) = match record.metadata().level() {
                Level::Error => ("error", Color::Red),
                Level::Warn => ("warn", Color::Magenta),
                Level::Info => ("info", Color::Yellow),
                Level::Debug => ("debug", Color::Cyan),
                Level::Trace => ("trace", Color::Blue),
            };

            let mut out = self.out.lock();

            if self.pretty {
                let mut color_spec = ColorSpec::new();
                color_spec.set_bold(true);
                color_spec.set_fg(Some(color));
                out.set_color(&color_spec).ok();
            }

            write!(out, "{}", name).unwrap();

            if self.pretty {
                out.reset().ok();
            }

            writeln!(out, ": {}", record.args()).unwrap();
        }

        fn flush(&self) {}
    }

    let pretty = atty::is(atty::Stream::Stderr);
    let logger = Logger {
        out: StandardStream::stderr(ColorChoice::Auto),
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
