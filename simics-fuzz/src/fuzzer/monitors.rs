use libafl::prelude::Monitor;

#[derive(Clone)]
pub struct MultiOrTui<T>(pub Box<T>)
where
    T: Monitor + Clone;

impl<T> Monitor for MultiOrTui<T>
where
    T: Monitor + Clone,
{
    fn client_stats_mut(&mut self) -> &mut Vec<libafl::prelude::ClientStats> {
        self.0.client_stats_mut()
    }

    fn client_stats(&self) -> &[libafl::prelude::ClientStats] {
        self.0.client_stats()
    }

    fn start_time(&mut self) -> std::time::Duration {
        self.0.start_time()
    }

    fn display(&mut self, event_msg: String, sender_id: libafl::prelude::ClientId) {
        self.0.display(event_msg, sender_id)
    }
}
