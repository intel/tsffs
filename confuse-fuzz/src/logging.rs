use anyhow::Result;
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    init_config, Handle,
};
use tempfile::Builder as NamedTempFileBuilder;

pub struct Logger {
    pub handle: Handle,
}

impl Logger {
    pub fn init() -> Result<Self> {
        let logfile = NamedTempFileBuilder::new()
            .prefix("confuse-log")
            .suffix(".log")
            .rand_bytes(4)
            .tempfile()?;
        let logfile_path = logfile.path().to_path_buf();
        let appender = FileAppender::builder()
            // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
            .encoder(Box::new(PatternEncoder::new(
                "[{h({l}):10.10}] | {d(%H:%M:%S)} | {m}{n}",
            )))
            .build(logfile_path)
            .unwrap();
        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(appender)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Trace),
            )
            .unwrap();
        let handle = init_config(config)?;

        Ok(Self { handle })
    }
}
