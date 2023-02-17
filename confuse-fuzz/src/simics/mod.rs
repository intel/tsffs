//! Simics Control and Configuration Interface for Confuse Fuzzer
//!
pub mod config;

use anyhow::Result;
use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};
use serde_yaml::to_string;
use signal_hook::{
    consts::signal::{SIGUSR1, SIGUSR2},
    iterator::{Handle, Signals},
};
use std::{
    path::PathBuf,
    process::{Child, Command},
    sync::{Arc, Condvar, Mutex},
    thread::{spawn, JoinHandle},
};

use self::config::SimicsConfig;

pub struct Simics {
    /// Path to the project directory
    project: PathBuf,
    /// SIMICS Child process
    simics: Option<Child>,
    /// Signal handler thread that is waiting for SIGUSR signals from the child process
    signal_handler_thread: Option<JoinHandle<()>>,
    /// Send + Sync Handle to the signal handling instance used for modifying the signal mask
    signal_handler_handle: Handle,
    /// Mutex set used to indicate a wait condition without busy waiting
    waiting: (Mutex<SimicsWaitState>, Condvar),
}

enum SimicsWaitState {
    NotWaiting = 0,
    Waiting = 1,
}

impl Simics {
    /// Create a new simics instance. This function will instantiate a new simics child process
    /// wrapped with a thread for signal handling of the ~~SIGUSR1~~ and SIGUSR2 signals, which are
    /// used to communicate from the child process back to this one.
    ///
    /// This function returns an Arc<Mutex<Simics>> because this structure is mutable across
    /// threads
    pub fn new(project: PathBuf) -> Result<Arc<Mutex<Self>>> {
        let mut signals = Signals::new(&[
            // SIGUSR1 is currently unused
            // SIGUSR1,
            SIGUSR2,
        ])?;
        let signal_handler_handle = signals.handle();

        let instance = Arc::new(Mutex::new(Self {
            project,
            simics: None,
            signal_handler_thread: None,
            signal_handler_handle,
            // Initialize to the "not waiting" state
            waiting: (Mutex::new(SimicsWaitState::NotWaiting), Condvar::new()),
        }));

        let signal_handler_instance = instance.clone();

        // This thread allows us to block on SIGUSR2 signal
        let signal_handler_thread = spawn(move || {
            loop {
                for signal in &mut signals {
                    match signal {
                        SIGUSR1 => {
                            // SIGUSR1 currently unused
                            // let mut simics = signal_handler_instance.lock().expect("Lock failed.");
                        }
                        SIGUSR2 => {
                            let simics = signal_handler_instance.lock().expect("Lock failed.");
                            // Notify waiting main thread that we've received signal from SIMICS
                            let mut pending = simics.waiting.0.lock().expect("Lock failed.");
                            *pending = SimicsWaitState::NotWaiting;
                            simics.waiting.1.notify_one();
                        }
                        _ => unreachable!(),
                    }
                }
                // We could active wait for a done message or something here but there is not much
                // need.
            }
        });

        instance.lock().expect("Lock failed").signal_handler_thread = Some(signal_handler_thread);

        Ok(instance)
    }

    /// Block the calling thread waiting for signal from the child process
    fn wait(&mut self) -> Result<()> {
        let (lock, val) = &self.waiting;

        // Release the thread until the mutex can lock and we are not waiting
        let _result = val
            .wait_while(lock.lock().expect("Lock failed"), |pending| match pending {
                SimicsWaitState::NotWaiting => false,
                SimicsWaitState::Waiting => true,
            })
            .expect("Could not wait.");
        Ok(())
    }

    /// Run the simulator by issuing SIGUSR1 and waiting for a return SIGUSR2
    pub fn run(&mut self) -> Result<()> {
        match &self.simics {
            Some(simics) => {
                signal::kill(Pid::from_raw(i32::try_from(simics.id())?), Signal::SIGUSR1)?;
                self.wait()?;
            }
            None => {}
        }

        Ok(())
    }

    /// Reset the simulator by issuing SIGUSR2 and waiting for a return SIGUSR2
    pub fn reset(&mut self) -> Result<()> {
        match &self.simics {
            Some(simics) => {
                signal::kill(Pid::from_raw(i32::try_from(simics.id())?), Signal::SIGUSR2)?;
                self.wait()?;
            }
            None => {}
        }

        Ok(())
    }

    pub fn init(&mut self, config: SimicsConfig) -> Result<()> {
        let simics_bin = self.project.join("bin").join("simics");

        self.simics = Some(
            Command::new(simics_bin)
                .arg(to_string(&config)?)
                .arg("-batch-mode")
                .arg("-e")
                .arg("@SIM_main_loop()")
                .spawn()?,
        );

        Ok(())
    }
}
