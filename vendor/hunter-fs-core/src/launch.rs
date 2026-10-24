//! Response-channel ownership and engine launch inside an existing guarded session.
use crate::{
    DirectoryTask, FilesystemBackend, ScanEngine, ScanHandler, ScanSession, ScanTasks,
    TraversalOutcome, WorkQueue,
};
use std::{path::Path, sync::mpsc, thread, time::Duration};

/// One response channel, allocated when the host chooses. The original
/// sender stays owned until coordinator entry; worker producers use cloned senders.
/// `new` retains the unbounded compatibility path; `bounded` applies backpressure.
/// A coordinator must drop the receiver before worker join, including on panic.
pub struct ScanChannel<Message, Sender = mpsc::Sender<Message>> {
    sender: Sender,
    receiver: mpsc::Receiver<Message>,
}

impl<Message> ScanChannel<Message> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }
}
impl<Message> ScanChannel<Message, mpsc::SyncSender<Message>> {
    /// Includes at most `capacity` queued messages (plus producer-owned in-flight
    /// payloads). Zero is a rendezvous channel. This does not bound payload bytes.
    pub fn bounded(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        Self { sender, receiver }
    }
}
impl<Message, Sender: Clone> ScanChannel<Message, Sender> {
    /// Clone a producer handle. The host must eventually release every producer.
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
}
impl<Message> Default for ScanChannel<Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowed engine inputs plus owned response channel. Construct after guard/backend
/// setup; allocate the channel separately to retain host startup/profile timing.
/// The backend and raw root are not owned, normalized, retried or moved to workers.
pub struct EngineLaunch<'a, Backend: ?Sized, Message, Preflight, Sender = mpsc::Sender<Message>> {
    pub backend: &'a Backend,
    pub root: &'a Path,
    pub channel: ScanChannel<Message, Sender>,
    pub preflight: Preflight,
    pub wake_interval: Option<Duration>,
}
impl<'env, Context: Send + 'env> ScanSession<'env, DirectoryTask<usize, Context>> {
    /// Start helpers/workers using the existing session, then enter the caller-thread
    /// coordinator: drop original sender, construct host, run ScanEngine, drop receiver,
    /// collect host/outcome, stop helpers, close queue and join before returning R.
    /// collect is before join (not terminal publication); it must release host-only
    /// state there if the original integration did so. R and the host need not be Send.
    ///
    /// A make_host/engine panic drops the owned receiver before scope join and skips
    /// collect. A collect panic also unwinds through helper/queue shutdown and join.
    /// A worker panic may be observed only at join, after collect has run; collect
    /// returning is not scan success and must not publish a terminal event.
    /// Existing session obligations apply: cooperative workers/helpers, valid response
    /// protocol, retained access guards, no guard-dependent handles escaping the body,
    /// and non-panicking destructors. No extra stop probe, I/O or thread is added.
    pub fn run_engine<B, H, P, W, S, G, F, C, R>(
        self,
        launch: EngineLaunch<'_, B, H::Message, P, impl Sized>,
        worker: W,
        start_helpers: S,
        make_host: F,
        collect: C,
    ) -> R
    where
        B: FilesystemBackend + ?Sized,
        H: ScanHandler<Context = Context>,
        P: FnOnce(&Path) -> Option<H::RootReason>,
        W: Fn(&WorkQueue<DirectoryTask<usize, Context>>) + Clone + Send + 'env,
        S: for<'scope> FnOnce(&'scope thread::Scope<'scope, 'env>) -> G,
        F: FnOnce() -> H,
        C: FnOnce(H, TraversalOutcome) -> R,
    {
        self.run(worker, start_helpers, move |queue| {
            let EngineLaunch {
                backend,
                root,
                channel,
                preflight,
                wake_interval,
            } = launch;
            let ScanChannel { sender, receiver } = channel;
            drop(sender);
            let mut host = make_host();
            let outcome = ScanEngine::new(&mut host, ScanTasks::<H>::new(queue)).run(
                backend,
                root,
                preflight,
                &receiver,
                wake_interval,
            );
            drop(receiver);
            collect(host, outcome)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_channel_rejects_an_extra_queued_message_and_drop_releases_producer() {
        let channel = ScanChannel::bounded(1);
        let sender = channel.sender();
        sender.try_send(1).unwrap();
        assert_eq!(sender.try_send(2), Err(mpsc::TrySendError::Full(2)));
        let ScanChannel {
            sender: original,
            receiver,
        } = channel;
        drop(original);
        thread::scope(|scope| {
            let worker = scope.spawn(move || sender.send(3));
            drop(receiver);
            assert_eq!(worker.join().unwrap().unwrap_err().0, 3);
        });
    }
}
