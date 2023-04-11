//! The main entrypoint of the confuse module when it is loaded in SIMICS. SIMICS first calls
//! `_simics_module_init` which is defined automatically by `generate_signature_header` and
//! included in this file. `_simics_module_init` then calls `init_local`, which is where we
//! perform our own initialization

use std::env::var;

use super::controller::Controller;
use confuse_simics_api::safe::{
    types::{ClassData, ConfClass, ConfObject},
    wrapper::register_class,
};
use const_format::concatcp;
use log::{info, Level, LevelFilter};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    init_config, Config,
};

pub const CLASS_NAME: &str = "confuse_module";
pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");

/// `confuse_init_local` is automatically called as the entrypoint of the module when it is loaded by
/// SIMICS. Components register initializers that are called by this function.
#[no_mangle]
pub extern "C" fn confuse_init_local() {
    Confuse::init().expect("Failed to initialize Confuse");
}

fn init_logging() -> Result<()> {
    let level = LevelFilter::from_str(
        &var(LOGLEVEL_VARNAME).unwrap_or_else(|_| Level::Trace.as_str().to_string()),
    )
    .unwrap_or(LevelFilter::Trace);
    let stderr = ConsoleAppender::builder()
        .target(Target::Stderr)
        // For SIMICS we just output the message because we're going to get stuck into a log
        // message anyway, and we need a newline or all the outputs will get buffered. lol
        .encoder(Box::new(PatternEncoder::new("[{l:5}] {m}{n}")))
        .build();
    // let level = LevelFilter::Info;
    let config = Config::builder()
        .appender(Appender::builder().build("stderr", Box::new(stderr)))
        .build(Root::builder().appender("stderr").build(level))?;
    let _log_handle = init_config(config)?;
    Ok(())
}

#[repr(C)]
/// Confuse module structure that we interface with SIMICs with
pub struct Confuse {
    /// The "object" SIMICS knows about relative to this module. When we recieve an `obj` pointer
    /// from SIMICS (usually in a Callback), we are receiving a pointer to this object in the
    /// struct
    obj: ConfObject,
    class: ConfClass,
    controller: Controller,
}

impl Confuse {
    pub fn init() {
        let class_data = ClassData {
            alloc_object: Some(alloc_controller_conf_object),
            init_object: Some(init_controller_conf_object),
            finalize_instance: None,
            pre_delete_instance: None,
            delete_instance: None,
            description: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
            class_desc: raw_cstr!(Controller::CLASS_DESCRIPTION),
            kind: class_kind_t_Sim_Class_Kind_Vanilla,
        };
        // let class_info = class_info_t {
        //     alloc: Some(alloc_controller_conf_object_for_create),
        //     init: Some(init_controller_conf_object_for_create),
        //     finalize: None,
        //     objects_finalized: None,
        //     deinit: None,
        //     dealloc: None,
        //     description: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
        //     short_desc: raw_cstr!(Controller::CLASS_DESCRIPTION),
        //     kind: class_kind_t_Sim_Class_Kind_Vanilla,
        // };

        info!("Creating class {}", Controller::CLASS_NAME);

        let cls = register_class(Controller::CLASS_NAME, class_data)?;
    }
}
