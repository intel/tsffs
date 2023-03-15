use crate::magic::Magic;
pub enum StopReason {
    Magic(Magic),
    Crash,
    Timeout,
}
