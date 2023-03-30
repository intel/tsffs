pub struct TargetBuffer {
    pub address: u64,
    pub size: u64,
}

impl Default for TargetBuffer {
    fn default() -> Self {
        Self {
            address: 0,
            size: 0,
        }
    }
}
