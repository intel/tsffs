use self::components::{detector::Detector, tracer::Tracer};
use crate::{
    client::{test::TestClient, Client},
    config::OutputConfig,
    interface::{ConfuseModuleInterface, Interface},
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
    stops::StopReason,
    traits::ConfuseClient,
    BOOTSTRAP_SOCKNAME, CLASS_NAME, TESTMODE_VARNAME,
};
use anyhow::{bail, Context, Result};
use const_format::concatcp;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use log::{debug, info, trace, Level, LevelFilter};
use raffl_macro::callback_wrappers;
use simics_api::{
    register_interface, ConfObject, OwnedMutAttrValuePtr, OwnedMutConfObjectPtr, SimicsLogger,
};
use simics_api::{Create, Module};
use simics_api_macro::module;
use std::{env::var, str::FromStr};

pub mod components;

pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");
pub const DEFAULT_LOGLEVEL: Level = Level::Trace;

#[module(class_name = CLASS_NAME)]
pub struct Confuse {
    /// In test mode, CONFUSE runs without a real client,
    test_mode_client: Option<Box<dyn ConfuseClient>>,
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    tracer: Tracer,
    detector: Detector,
}

impl Module for Confuse {
    fn init(module_instance: OwnedMutConfObjectPtr) -> Result<OwnedMutConfObjectPtr> {
        let log_level = LevelFilter::from_str(&var(LOGLEVEL_VARNAME).unwrap_or_default())
            .unwrap_or(DEFAULT_LOGLEVEL.to_level_filter());

        let test_mode = if let Ok(name) = var(TESTMODE_VARNAME) {
            match name.to_ascii_lowercase().as_str() {
                "1" => true,
                "true" => true,
                "on" => true,
                _ => false,
            }
        } else {
            false
        };

        SimicsLogger::new()
            // Dev is a misnomer here -- that's what SIMICS calls it but really it should just be
            // `object` because that's all we are doing here is creating a logger for our module object
            .with_dev(module_instance.clone())
            .with_level(log_level)
            .init()?;

        let state = State::new();
        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;
        let test_client = if test_mode {
            Some(TestClient::new_boxed(otx, orx))
        } else {
            info!("Initializing CONFUSE");

            let sockname = var(BOOTSTRAP_SOCKNAME)?;
            debug!("Connecting to bootstrap socket {}", sockname);

            let bootstrap = IpcSender::connect(sockname)?;

            debug!("Connected to bootstrap socket");

            bootstrap.send((otx, orx))?;

            debug!("Sent primary socket over bootstrap socket");

            None
        };
        let detector = Detector::try_new()?;
        let tracer = Tracer::try_new()?;

        Ok(Confuse::new(
            module_instance,
            test_client,
            state,
            tx,
            rx,
            tracer,
            detector,
        ))
    }

    fn objects_finalized(module_instance: OwnedMutConfObjectPtr) -> Result<()> {
        let confuse: &mut Confuse = module_instance.into();
        let config = match confuse.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        let mut output_config = OutputConfig::default();

        confuse.send_msg(ModuleMessage::Initialized(output_config))?;

        Ok(())
    }
}

impl Confuse {
    /// Send a message to the client
    fn send_msg(&mut self, msg: ModuleMessage) -> Result<()> {
        trace!("Sending module message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming sent message {:?}", msg))?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the client
    fn recv_msg(&mut self) -> Result<ClientMessage> {
        trace!("Waiting to receive client message");
        let msg = self.rx.recv()?;
        trace!("Received client message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming received message {:?}", msg))?;
        Ok(msg)
    }
}

impl Confuse {
    pub fn stop_simulation(&mut self, reason: StopReason) -> Result<()> {
        Ok(())
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl Confuse {
    #[params()]
    pub fn on_simulation_stopped(
        &mut self,
        _data: *mut c_void,
        _trigger_obj: *mut conf_object_t,
        // Exception is always SimExc_No_Exception
        _exception: i64,
        // Error string is always NULL
        _error_string: *mut c_char,
    ) -> Result<()> {
        Ok(())
    }

    #[params()]
    pub fn on_magic_instruction(
        &mut self,
        _user_data: *mut c_void,
        _trigger_obj: *const conf_object_t,
        parameter: i64,
    ) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    let cls = Confuse::create().unwrap_or_else(|_| panic!("Failed to create class {}", CLASS_NAME));

    register_interface::<_, ConfuseModuleInterface>(cls, CLASS_NAME)
        .unwrap_or_else(|_| panic!("Failed to register interface for class {}", CLASS_NAME));
}
