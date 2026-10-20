//! Response-channel ownership and engine launch inside an existing guarded session.
use crate::{
    DirectoryTask, FilesystemBackend, ScanEngine, ScanHandler, ScanSession, ScanTasks,
    TraversalOutcome, WorkQueue,
};
use std::{path::Path, sync::mpsc, thread, time::Duration};

/// One unbounded response channel, allocated when the host chooses. The original
/// sender stays owned until coordinator entry; worker producers use cloned senders.
/// Neither message interpretation nor resource bounds are supplied by this owner.
pub struct ScanChannel<Message> {
    sender: mpsc::Sender<Message>,
    receiver: mpsc::Receiver<Message>,
}
impl<Message> ScanChannel<Message> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }
    /// Clone a producer handle. The host must eventually release every producer.
    pub fn sender(&self) -> mpsc::Sender<Message> {
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
pub struct EngineLaunch<'a, Backend: ?Sized, Message, Preflight> {
    pub backend: &'a Backend,
    pub root: &'a Path,
    pub channel: ScanChannel<Message>,
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
        launch: EngineLaunch<'_, B, H::Message, P>,
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
