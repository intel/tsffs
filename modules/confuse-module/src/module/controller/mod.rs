use self::messages::{client::ClientMessage, module::ModuleMessage};
use crate::module::entrypoint::{BOOTSTRAP_SOCKNAME, LOGLEVEL_VARNAME};
use anyhow::Result;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use lazy_static::lazy_static;
use log::{info, Level, LevelFilter};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    init_config, Handle,
};
use std::{
    env::var,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
};

use super::component::Component;

pub mod magic;
pub mod message;
pub mod messages;

lazy_static! {
    pub static ref CONTROLLER: Arc<Mutex<Controller>> = Arc::new(Mutex::new(
        Controller::try_new().expect("Could not initialize Controller")
    ));
}

/// Controller for the Confuse simics module. The controller is reponsible for communicating with
/// the client, dispatching messages, and implementing the overall state machine for the module
pub struct Controller {
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    log_handle: Handle,
}

impl Controller {
    pub fn get<'a>() -> Result<MutexGuard<'a, Self>> {
        let controller = CONTROLLER.lock().expect("Could not lock controller");
        Ok(controller)
    }
}

impl Controller {
    pub fn try_new() -> Result<Self> {
        let level = LevelFilter::from_str(
            &var(LOGLEVEL_VARNAME).unwrap_or_else(|_| Level::Trace.as_str().to_string()),
        )
        .unwrap_or(LevelFilter::Trace);
        let stderr = ConsoleAppender::builder()
            .target(Target::Stderr)
            // For SIMICS we just output the message because we're going to get stuck into a log
            // message anyway, and we need a newline or all the outputs will get buffered. lol
            .encoder(Box::new(PatternEncoder::new("[SIMICS] {m}{n}")))
            .build();
        // let level = LevelFilter::Info;
        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .build(Root::builder().appender("stderr").build(level))?;
        let log_handle = init_config(config)?;

        let sockname = var(BOOTSTRAP_SOCKNAME)?;
        let bootstrap = IpcSender::connect(sockname)?;

        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;

        bootstrap.send((otx, orx))?;

        Ok(Self { tx, rx, log_handle })
    }

    pub fn send(&self, message: ModuleMessage) -> Result<()> {
        self.tx.send(message)?;
        Ok(())
    }
}

impl Component for Controller {
    fn on_initialize(
        &mut self,
        initialize_config: super::config::InitializeConfig,
        initialized_config: &mut super::config::InitializedConfig,
    ) -> Result<()> {
        todo!()
    }

    fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        todo!()
    }

    fn on_reset(&mut self) -> Result<()> {
        todo!()
    }

    fn on_stop(&mut self) -> Result<()> {
        todo!()
    }
}
