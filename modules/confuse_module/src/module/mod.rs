use self::components::{detector::Detector, tracer::Tracer};
use crate::{
    config::OutputConfig,
    interface::{ConfuseModuleInterface, Interface},
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
    stops::StopReason,
    BOOTSTRAP_SOCKNAME, CLASS_NAME,
};
use anyhow::{bail, Context, Result};
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use log::{debug, info, trace, Level};
use raffl_macro::callback_wrappers;
use simics_api::{
    register_interface, ConfObject, OwnedMutAttrValuePtr, OwnedMutConfObjectPtr, SimicsLogger,
};
use simics_api::{Create, Module};
use simics_api_derive::module;
use std::env::var;

pub mod components;

#[module(class_name = CLASS_NAME)]
pub struct Confuse {
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    tracer: Tracer,
    detector: Detector,
}

impl Module for Confuse {
    fn init(module_instance: OwnedMutConfObjectPtr) -> Result<OwnedMutConfObjectPtr> {
        SimicsLogger::new()
            // Dev is a misnomer here -- that's what SIMICS calls it but really it should just be
            // `object` because that's all we are doing here is creating a logger for our module object
            .with_dev(module_instance.clone())
            .with_level(Level::Trace.to_level_filter())
            .init()?;

        info!("Initializing CONFUSE");

        let state = State::new();
        let sockname = var(BOOTSTRAP_SOCKNAME)?;

        debug!("Connecting to bootstrap socket {}", sockname);

        let bootstrap = IpcSender::connect(sockname)?;

        debug!("Connected to bootstrap socket");

        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;

        bootstrap.send((otx, orx))?;

        debug!("Sent primary socket over bootstrap socket");

        let detector = Detector::try_new()?;
        let tracer = Tracer::try_new()?;

        Ok(Confuse::new(
            module_instance,
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

    #[params()]
    pub fn 
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    let cls = Confuse::create().unwrap_or_else(|_| panic!("Failed to create class {}", CLASS_NAME));

    register_interface::<_, ConfuseModuleInterface>(cls, CLASS_NAME)
        .unwrap_or_else(|_| panic!("Failed to register interface for class {}", CLASS_NAME));
}
