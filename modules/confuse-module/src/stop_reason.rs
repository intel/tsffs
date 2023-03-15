use confuse_fuzz::Fault;

use crate::magic::Magic;
pub enum StopReason {
    Magic(Magic),
    Crash(Fault),
    Timeout,
}
