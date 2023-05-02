extern "C" {
    /// Discard recorded future events and forget them
    pub fn CORE_discard_future();
}

pub fn discard_future() {
    unsafe { CORE_discard_future() };
}
