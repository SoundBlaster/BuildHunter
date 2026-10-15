//! Serial coordinator bookkeeping; caller owns result protocol and worker lifetime.
use crate::{DirectoryRequest, DirectoryTask};
use std::collections::HashMap;

/// Holds opaque listing state and deferred measurement state keyed by non-reused IDs.
/// Not a scheduler, channel, protocol validator or join boundary. The host records
/// each received message before taking its result state and supplies valid IDs/order.
/// Arena capacity and HashMap capacity are retained until drop; no memory bound.
#[derive(Debug)]
pub struct DirectoryTaskLedger<State, Pending> {
    listings: Vec<Option<State>>,
    measurements: HashMap<usize, Option<Pending>>,
    outstanding: usize,
}
impl<State, Pending> Default for DirectoryTaskLedger<State, Pending> {
    fn default() -> Self {
        Self {
            listings: Vec::new(),
            measurements: HashMap::new(),
            outstanding: 0,
        }
    }
}
impl<State, Pending> DirectoryTaskLedger<State, Pending> {
    /// Reserve an ID/state and one listing response before the host schedules the task.
    /// Rejected dispatch handling and association of request with state are host-owned.
    pub fn register<Context>(
        &mut self,
        request: DirectoryRequest<Context>,
        state: State,
    ) -> DirectoryTask<usize, Context> {
        let task = request.into_task(self.listings.len());
        self.listings.push(Some(state));
        self.outstanding += 1;
        task
    }

    /// Number of expected result messages, including admitted measurement responses.
    pub fn outstanding(&self) -> usize {
        self.outstanding
    }

    /// Retire one expected message on receipt, before host cancellation/dispatch checks.
    /// A host may receive a setup failure instead of the expected listing and stop.
    /// Panics if no work was outstanding; this does not validate a message ID/type.
    pub fn response_received(&mut self) {
        self.outstanding = self
            .outstanding
            .checked_sub(1)
            .expect("response requires outstanding work");
    }

    /// Move out state for the first valid listing and reserve its measurement response.
    /// Host calls response_received first, then chooses whether/how to defer measurement.
    /// Panics on an unknown/already consumed listing ID. IDs are never recycled.
    pub fn take_listing(&mut self, id: usize, measuring: bool) -> State {
        let state = self
            .listings
            .get_mut(id)
            .and_then(Option::take)
            .expect("each directory is listed once");
        if measuring {
            self.outstanding += 1;
        }
        state
    }

    /// Host records at most once per ID, after a listing announcing measurement and
    /// before consuming that response. None explicitly ignores speculative work.
    /// No counter change: take_listing already reserved the response. Payloads are
    /// opaque; this method does not validate this host protocol or acquire resources.
    pub fn defer_measurement(&mut self, id: usize, pending: Option<Pending>) {
        self.measurements.insert(id, pending);
    }

    /// Move deferred state to the host, or ignore an unneeded/unmatched response.
    /// Duplicate/unknown IDs yield None, preserving the original consumer behavior.
    /// Call response_received separately; this lookup never changes response count.
    pub fn take_measurement(&mut self, id: usize) -> Option<Pending> {
        self.measurements.remove(&id).flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DirectoryRequest;
    use std::{cell::Cell, path::PathBuf, rc::Rc};
    fn request(path: &str) -> DirectoryRequest<u32> {
        DirectoryRequest {
            path: PathBuf::from(path),
            context: 17,
            measure: true,
        }
    }
    #[test]
    fn registration_assigns_nonreused_ids_and_retains_request_values() {
        let mut ledger = DirectoryTaskLedger::<&str, ()>::default();
        let first = ledger.register(request("first"), "state-a");
        let second = ledger.register(request("second"), "state-b");
        assert_eq!((first.id, second.id), (0, 1));
        assert_eq!(first.path, PathBuf::from("first"));
        assert_eq!(first.context, 17);
        assert!(first.measure);
        assert_eq!(ledger.outstanding(), 2);
        ledger.response_received();
        assert_eq!(ledger.take_listing(second.id, false), "state-b");
        ledger.response_received();
        assert_eq!(ledger.take_listing(first.id, false), "state-a");
        assert_eq!(ledger.outstanding(), 0);
        assert_eq!(ledger.register(request("third"), "state-c").id, 2);
    }
    #[test]
    fn listed_and_measured_messages_keep_separate_response_counts() {
        let mut ledger = DirectoryTaskLedger::default();
        let a = ledger.register(request("a"), "listed-a").id;
        let b = ledger.register(request("b"), "listed-b").id;
        ledger.response_received();
        assert_eq!(ledger.outstanding(), 1);
        assert_eq!(ledger.take_listing(a, true), "listed-a");
        assert_eq!(ledger.outstanding(), 2);
        ledger.defer_measurement(a, Some("measured-a"));
        ledger.response_received();
        assert_eq!(ledger.take_listing(b, true), "listed-b");
        ledger.defer_measurement(b, Some("measured-b"));
        assert_eq!(ledger.outstanding(), 2);
        ledger.response_received();
        assert_eq!(ledger.take_measurement(b), Some("measured-b"));
        ledger.response_received();
        assert_eq!(ledger.take_measurement(a), Some("measured-a"));
        assert_eq!(ledger.outstanding(), 0);
    }
    #[test]
    fn ignored_measurement_still_needs_a_response_but_has_no_payload() {
        let mut ledger = DirectoryTaskLedger::<(), ()>::default();
        let id = ledger.register(request("a"), ()).id;
        ledger.response_received();
        ledger.take_listing(id, true);
        ledger.defer_measurement(id, None);
        assert_eq!(ledger.outstanding(), 1);
        ledger.response_received();
        assert_eq!(ledger.take_measurement(id), None);
        assert_eq!(ledger.take_measurement(id), None);
        assert_eq!(ledger.take_measurement(999), None);
        assert_eq!(ledger.outstanding(), 0);
    }
    struct Payload(Rc<Cell<usize>>);
    impl Drop for Payload {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    #[test]
    fn dropping_stopped_ledger_releases_unclaimed_listing_and_pending_payloads_once() {
        let drops = Rc::new(Cell::new(0));
        let mut ledger = DirectoryTaskLedger::default();
        let a = ledger.register(request("a"), Payload(drops.clone())).id;
        let _b = ledger.register(request("b"), Payload(drops.clone())).id;
        ledger.response_received();
        drop(ledger.take_listing(a, true));
        assert_eq!(drops.get(), 1);
        ledger.defer_measurement(a, Some(Payload(drops.clone())));
        assert_eq!(ledger.outstanding(), 2);
        drop(ledger);
        assert_eq!(drops.get(), 3);
    }
    #[test]
    fn setup_failure_receipt_leaves_unclaimed_state_for_drop() {
        let drops = Rc::new(Cell::new(0));
        let mut ledger = DirectoryTaskLedger::<Payload, ()>::default();
        ledger.register(request("a"), Payload(drops.clone()));
        ledger.response_received();
        assert_eq!(ledger.outstanding(), 0);
        assert_eq!(drops.get(), 0);
        drop(ledger);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    #[should_panic(expected = "response requires outstanding work")]
    fn response_without_outstanding_work_is_a_host_protocol_error() {
        DirectoryTaskLedger::<(), ()>::default().response_received();
    }
    #[test]
    #[should_panic(expected = "each directory is listed once")]
    fn duplicate_listing_is_a_host_protocol_error() {
        let mut ledger = DirectoryTaskLedger::<(), ()>::default();
        let id = ledger.register(request("a"), ()).id;
        ledger.response_received();
        ledger.take_listing(id, false);
        ledger.take_listing(id, false);
    }
}
