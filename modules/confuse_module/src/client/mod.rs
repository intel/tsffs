//! The CONFUSE module client provides a common client-side controller for a fuzzer or other tool
//! to communicate with the module while keeping consistent with the state machine the module
//! implements.
//!
//! This client is designed to be used with the [`confuse-fuzz`] crate, but can be used manually as
//! well to implement bespoke systems.
//!
//! # Examples
//!
//! In this example, we show what an extremely basic fuzz loop might look like
//! without using LibAFL. This loop is consistent with the state machine used
//! internally by the client and module that keeps them in sync.
//!
//! ```
//! let simics_script_path = PathBuf::from("./script.simics");
//! let project = SimicsProject::try_new_latest()?
//!     // Add a file to the created simics project at `PROJECT_ROOT/scripts/script.simics`
//!     .try_with_file(&simics_script_path, "scripts/script.simics")?
//!     // This script will be our entrypoint when we run SIMICS
//!     .try_with_file_argument("scripts/script.simics")?;
//!
//! // Create a client that owns the project we just created
//! let mut client = Client::try_new(project)?;
//!
//! // Create a blank configuration
//! let config = InputConfig::default();
//!
//! // Initialize the client. This takes us up to the point where the module is ready
//! // to start the fuzzing loop
//! let output_config = client.initialize(config)?;
//!
//!
//! for _ in 0..100 {
//!     // Reset the target to its initial state once it has been initialized. We could also
//!     client.reset()?;
//!     // Run the target with the same input every time. In real life, we want to
//!     // swap this out with a fuzzer, of course
//!     let stop_reason = client.run(vec![0x41; 64])?;
//! }
//!
//! // Cleanly exit SIMICS and stop the client
//! client.exit()?;
//! ```

use crate::{
    module::{
        config::{InputConfig, OutputConfig},
        controller::messages::{client::ClientMessage, module::ModuleMessage},
        entrypoint::{BOOTSTRAP_SOCKNAME, CLASS_NAME, LOGLEVEL_VARNAME},
        stop_reason::StopReason,
    },
    state::State,
};
use anyhow::{bail, Result};
use confuse_simics_project::SimicsProject;
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use log::{debug, info};
use std::io::{BufRead, BufReader};

/// The client for the CONFUSE module. Allows controlling the module over IPC using the child
/// process spawned by a running project.
pub struct Client {
    /// State machine to keep track of the current state between the client and module
    state: State,
    /// Transmit end of IPC message channel between client and module
    tx: IpcSender<ClientMessage>,
    /// Receive end of IPC message channel between client and module
    rx: IpcReceiver<ModuleMessage>,
    /// The SIMICS project owned by this client
    pub project: SimicsProject,
}

impl Client {
    /// Try to initialize a `Client` from a built `SimicsProject` on disk, which should include
    /// the CONFUSE module and may have additional configuration according to user needs. Creating
    /// the client will start the SIMICS project, which should be configured as necessary *before*
    /// passing it into this constructor.
    ///
    /// The CONFUSE Simics module will be added to the project for you if it is not present,
    /// so
    pub fn try_new(mut project: SimicsProject) -> Result<Self> {
        // Make sure the project has our module loaded in it
        if !project.has_module(CLASS_NAME) {
            info!("Project is missing module {}, adding it", CLASS_NAME);
            project = project.try_with_module(CLASS_NAME)?;
        }

        let (bootstrap, bootstrap_name) = IpcOneShotServer::new()?;

        // Set a few environment variables for the project and add output loggers
        let loglevel_str = project.loglevel.as_str();

        project = project
            .with_batch_mode(true)
            .with_command("@SIM_main_loop()")
            .with_env(BOOTSTRAP_SOCKNAME, &bootstrap_name)
            .with_env(LOGLEVEL_VARNAME, loglevel_str)
            .with_stdout_function(|stdout| {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();

                info!("Starting stdout reader");

                loop {
                    line.clear();
                    let rv = reader.read_line(&mut line).expect("Could not read line");
                    if rv == 0 {
                        break;
                    }
                    let logline = line.trim();
                    if !logline.is_empty() {
                        info!("[SIMICS OUT] {}", line.trim());
                    }
                }
                info!("Output reader exited.");
            })
            .with_stderr_function(|stderr| {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();

                debug!("Starting stderr reader");

                loop {
                    line.clear();
                    let rv = reader.read_line(&mut line).expect("Could not read line");
                    if rv == 0 {
                        break;
                    }
                    let logline = line.trim();
                    if !logline.is_empty() {
                        debug!("[SIMICS ERR] {}", line.trim());
                    }
                }
                info!("Err reader exited.");
            });

        project = project.build()?;

        project.run()?;

        let (_, (tx, rx)): (_, (IpcSender<ClientMessage>, IpcReceiver<ModuleMessage>)) =
            bootstrap.accept()?;

        Ok(Self {
            state: State::new(),
            tx,
            rx,
            project,
        })
    }

    /// Initialize the client with a configuration. The client will return an output
    /// configuration which contains various information the SIMICS module needs to
    /// inform the client of, including memory maps for coverage. Changes the
    /// internal state from [`Uninitialized`] to [`HalfInitialized`] and then from
    /// [`HalfInitialized`] to [`ConfuseModuleState::Initialized`].
    pub fn initialize(&mut self, config: InputConfig) -> Result<OutputConfig> {
        info!("Sending initialize message");
        self.send_msg(ClientMessage::Initialize(config))?;

        info!("Waiting for initialized message");
        if let ModuleMessage::Initialized(config) = self.recv_msg()? {
            Ok(config)
        } else {
            bail!("Initialization failed, received unexpected message");
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        info!("Sending reset message");
        self.send_msg(ClientMessage::Reset)?;

        info!("Waiting for ready message");
        if let ModuleMessage::Ready = self.recv_msg()? {
            Ok(())
        } else {
            bail!("Reset failed, received unexpected message");
        }
    }

    pub fn run(&mut self, input: Vec<u8>) -> Result<StopReason> {
        info!("Sending run message");
        self.send_msg(ClientMessage::Run(input))?;

        info!("Waiting for stopped message");
        if let ModuleMessage::Stopped(reason) = self.recv_msg()? {
            Ok(reason)
        } else {
            bail!("Run failed, received unexpected message");
        }
    }

    pub fn exit(&mut self) -> Result<()> {
        info!("Sending exit message");
        self.send_msg(ClientMessage::Exit)?;

        info!("Killing SIMICS");
        self.project.kill()?;

        Ok(())
    }

    /// Send a message to the module
    fn send_msg(&mut self, msg: ClientMessage) -> Result<()> {
        self.state.consume(&msg)?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the module
    fn recv_msg(&mut self) -> Result<ModuleMessage> {
        let msg = self.rx.recv()?;
        self.state.consume(&msg)?;
        Ok(msg)
    }
}
