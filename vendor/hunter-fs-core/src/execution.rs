//! Serial scan result pump; the host owns filesystem/policy and scoped worker lifetime.
use std::{sync::mpsc, time::Duration};

/// Host-owned state for one synchronous result loop. Callbacks execute on the
/// calling thread and need not be Send or 'static. Message values move once.
pub trait ScanCoordinator {
    type Message;
    fn outstanding(&self) -> usize;
    fn should_stop(&self) -> bool;
    /// Called before each wait, including after an idle timeout.
    fn checkpoint(&mut self);
    /// Retire a received response and update host telemetry before the stop check.
    fn response_received(&mut self);
    /// Interpret the message, optionally announcing/scheduling additional work.
    fn dispatch(&mut self, message: Self::Message);
}

/// Why the pump returned; this is not a scan status or proof of worker shutdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinatorExit {
    Drained,
    Stopped,
    Disconnected,
}

/// Drive an already initialized scan's serial result loop. Empty work wins over
/// a stop at the loop boundary. Checkpoint, receive, retire, stop check, dispatch
/// preserve that exact order. A timeout only repeats the loop/checkpoint; channel
/// disconnection leaves outstanding accounting untouched. Dispatch can add work.
///
/// None blocks until a response/disconnection. A positive interval permits idle
/// telemetry and stop checks, without interrupting syscalls or host callbacks.
/// A stop triggered by checkpoint is checked after receipt (or the next timeout),
/// preserving the consumer's existing cancellation boundary. The host must close
/// its queue and stop helpers before its outer Scope joins, including on panic.
/// Root setup, partial finalization, errors/status/events and memory budgets are
/// host-owned; this function neither spawns workers nor owns their resources.
pub fn drive_scan<C: ScanCoordinator>(
    host: &mut C,
    receiver: &mpsc::Receiver<C::Message>,
    wake_interval: Option<Duration>,
) -> CoordinatorExit {
    assert!(
        wake_interval.is_none_or(|interval| !interval.is_zero()),
        "wake interval must be positive"
    );
    loop {
        if host.outstanding() == 0 {
            return CoordinatorExit::Drained;
        }
        if host.should_stop() {
            return CoordinatorExit::Stopped;
        }
        host.checkpoint();
        let message = if let Some(interval) = wake_interval {
            match receiver.recv_timeout(interval) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return CoordinatorExit::Disconnected,
            }
        } else {
            match receiver.recv() {
                Ok(message) => message,
                Err(_) => return CoordinatorExit::Disconnected,
            }
        };
        host.response_received();
        if host.should_stop() {
            return CoordinatorExit::Stopped;
        }
        host.dispatch(message);
    }
}

#[cfg(test)]
mod tests {
    use crate::{CoordinatorExit, ScanCoordinator, drive_scan};
    use std::{sync::mpsc, time::Duration};

    #[derive(Default)]
    struct Host {
        outstanding: usize,
        stopped: bool,
        checkpoints: usize,
        received: usize,
        handled: Vec<usize>,
        stop_on_receipt: bool,
        stop_on_checkpoint: bool,
        expand: Option<mpsc::Sender<usize>>,
        send_after_timeout: Option<mpsc::Sender<usize>>,
        panic_on_dispatch: bool,
    }
    impl ScanCoordinator for Host {
        type Message = usize;
        fn outstanding(&self) -> usize {
            self.outstanding
        }
        fn should_stop(&self) -> bool {
            self.stopped
        }
        fn checkpoint(&mut self) {
            self.checkpoints += 1;
            if self.stop_on_checkpoint {
                self.stopped = true;
            }
            if self.checkpoints == 2
                && let Some(sender) = self.send_after_timeout.take()
            {
                sender.send(7).unwrap();
            }
        }
        fn response_received(&mut self) {
            self.outstanding -= 1;
            self.received += 1;
            if self.stop_on_receipt {
                self.stopped = true;
            }
        }
        fn dispatch(&mut self, message: usize) {
            assert_eq!(
                self.received,
                self.handled.len() + 1,
                "retire before dispatch"
            );
            assert!(!self.panic_on_dispatch, "injected callback panic");
            self.handled.push(message);
            if message == 0
                && let Some(sender) = &self.expand
            {
                self.outstanding += 2;
                sender.send(1).unwrap();
                sender.send(2).unwrap();
            }
        }
    }

    #[test]
    fn empty_work_does_not_checkpoint_receive_or_check_stop() {
        let (tx, rx) = mpsc::channel();
        tx.send(9).unwrap();
        let mut host = Host {
            stopped: true,
            ..Host::default()
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Drained);
        assert_eq!((host.checkpoints, host.received), (0, 0));
        assert_eq!(rx.try_recv().unwrap(), 9);
    }
    #[test]
    fn precancelled_work_preserves_the_queued_response() {
        let (tx, rx) = mpsc::channel();
        tx.send(9).unwrap();
        let mut host = Host {
            outstanding: 1,
            stopped: true,
            ..Host::default()
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Stopped);
        assert_eq!(
            (host.checkpoints, host.received, host.outstanding),
            (0, 0, 1)
        );
        assert_eq!(rx.try_recv().unwrap(), 9);
    }
    #[test]
    fn dispatch_can_schedule_more_work_before_the_next_termination_check() {
        let (tx, rx) = mpsc::channel();
        tx.send(0).unwrap();
        let mut host = Host {
            outstanding: 1,
            expand: Some(tx),
            ..Host::default()
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Drained);
        assert_eq!(host.handled, [0, 1, 2]);
        assert_eq!(
            (host.checkpoints, host.received, host.outstanding),
            (3, 3, 0)
        );
    }
    #[test]
    fn stop_after_receipt_retires_but_does_not_dispatch_the_message() {
        let (tx, rx) = mpsc::channel();
        tx.send(4).unwrap();
        let mut host = Host {
            outstanding: 1,
            stop_on_receipt: true,
            ..Host::default()
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Stopped);
        assert_eq!((host.received, host.outstanding), (1, 0));
        assert!(host.handled.is_empty());
    }
    #[test]
    fn checkpoint_stop_keeps_the_existing_receive_then_retire_boundary() {
        let (tx, rx) = mpsc::channel();
        tx.send(4).unwrap();
        let mut host = Host {
            outstanding: 1,
            stop_on_checkpoint: true,
            ..Host::default()
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Stopped);
        assert_eq!(
            (host.checkpoints, host.received, host.outstanding),
            (1, 1, 0)
        );
        assert!(host.handled.is_empty());
    }
    #[test]
    fn timeout_repeats_checkpoint_without_retiring_a_response() {
        let (tx, rx) = mpsc::channel();
        let mut host = Host {
            outstanding: 1,
            send_after_timeout: Some(tx),
            ..Host::default()
        };
        assert_eq!(
            drive_scan(&mut host, &rx, Some(Duration::from_millis(1))),
            CoordinatorExit::Drained
        );
        assert_eq!((host.checkpoints, host.received), (2, 1));
        assert_eq!(host.handled, [7]);
    }
    #[test]
    fn disconnection_preserves_unanswered_count_in_both_wait_modes() {
        for interval in [None, Some(Duration::from_millis(1))] {
            let (tx, rx) = mpsc::channel();
            drop(tx);
            let mut host = Host {
                outstanding: 2,
                ..Host::default()
            };
            assert_eq!(
                drive_scan(&mut host, &rx, interval),
                CoordinatorExit::Disconnected
            );
            assert_eq!(
                (host.checkpoints, host.received, host.outstanding),
                (1, 0, 2)
            );
        }
    }
    #[test]
    fn callback_panic_propagates_to_the_host_scope() {
        let (tx, rx) = mpsc::channel();
        tx.send(1).unwrap();
        let mut host = Host {
            outstanding: 1,
            panic_on_dispatch: true,
            ..Host::default()
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drive_scan(&mut host, &rx, None)
        }));
        assert!(result.is_err());
        assert_eq!(host.received, 1);
    }

    #[test]
    #[should_panic(expected = "wake interval must be positive")]
    fn zero_interval_is_rejected_instead_of_busy_polling() {
        let (_tx, rx) = mpsc::channel();
        drive_scan(&mut Host::default(), &rx, Some(Duration::ZERO));
    }

    #[test]
    fn host_and_messages_can_borrow_non_send_state() {
        use std::{cell::Cell, rc::Rc};
        struct Borrowed<'a, 'b> {
            remaining: Rc<Cell<usize>>,
            seen: &'b mut Vec<&'a str>,
        }
        impl<'a> ScanCoordinator for Borrowed<'a, '_> {
            type Message = &'a str;
            fn outstanding(&self) -> usize {
                self.remaining.get()
            }
            fn should_stop(&self) -> bool {
                false
            }
            fn checkpoint(&mut self) {}
            fn response_received(&mut self) {
                self.remaining.set(self.remaining.get() - 1);
            }
            fn dispatch(&mut self, message: &'a str) {
                self.seen.push(message);
            }
        }
        let message = String::from("borrowed");
        let (tx, rx) = mpsc::channel();
        tx.send(message.as_str()).unwrap();
        let mut seen = Vec::new();
        let remaining = Rc::new(Cell::new(1));
        let mut host = Borrowed {
            remaining: remaining.clone(),
            seen: &mut seen,
        };
        assert_eq!(drive_scan(&mut host, &rx, None), CoordinatorExit::Drained);
        assert_eq!(remaining.get(), 0);
        assert_eq!(seen, ["borrowed"]);
    }
}
