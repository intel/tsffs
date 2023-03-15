//! Confuse module interface to simics -- this just defines the boilerplate needed for it to
//! be loaded as a SIMICs module
include!(concat!(env!("OUT_DIR"), "/simics_module_header.rs"));

use anyhow::Result;
use const_format::concatcp;
use log::{info, LevelFilter};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    init_config,
};

use crate::context::CTX;

pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");

fn init_logging() -> Result<()> {
    let stderr = ConsoleAppender::builder()
        .target(Target::Stderr)
        // For SIMICS we just output the message because we're going to get stuck into a log
        // message anyway, and we need a newline or all the outputs will get buffered. lol
        .encoder(Box::new(PatternEncoder::new("[SIMICS] {m}{n}")))
        .build();
    // let level = LevelFilter::Info;
    let config = Config::builder()
        .appender(Appender::builder().build("stderr", Box::new(stderr)))
        .build(Root::builder().appender("stderr").build(LevelFilter::Trace))
        .unwrap();
    let _handle = init_config(config)?;
    Ok(())
}

#[no_mangle]
pub extern "C" fn init_local() {
    init_logging().expect("Could not initialize logging");
    let mut ctx = CTX.lock().expect("Could not lock context!");
    ctx.init().expect("Could not initialize context");
    info!("Initialized context for {}", CLASS_NAME);
}
