//! Synchronous directory-task protocol; filesystem and prefetch policy stay in the host.
use crate::{
    DirectoryListing, DirectoryMeasurement, DirectoryMeasurementPlan, DirectoryResult,
    DirectoryTask, WorkQueue,
};

/// Borrowed, statically dispatched host effects. No Send/Sync or allocation requirement
/// is added here; the caller owns threading, guards, cancellation and error decoration.
pub trait DirectoryWorker {
    type Id: Clone;
    type Context;

    fn list(
        &mut self,
        task: &DirectoryTask<Self::Id, Self::Context>,
    ) -> DirectoryListing<Self::Context>;
    fn prefetch(
        &mut self,
        task: &DirectoryTask<Self::Id, Self::Context>,
        listing: &DirectoryListing<Self::Context>,
    ) -> bool;
    /// Execute the supplied plan, retaining its ID and original-position result slots.
    fn measure(
        &mut self,
        plan: DirectoryMeasurementPlan<'_, Self::Id>,
    ) -> DirectoryMeasurement<Self::Id>;
    /// Return false when the recipient rejects this result; no further work is taken.
    fn publish(&mut self, result: DirectoryResult<Self::Id, Self::Context>) -> bool;
}

/// Pop tasks until queue closure or rejected publication. Publish Listed before invoking
/// measurement, then publish even a cancelled/partial measurement. Host owns cancellation
/// inside callbacks: there is deliberately no additional stop check between these steps.
/// Rejection does not close the shared queue; caller owns shutdown and scope join. Panics
/// propagate, so scoped workers must retain their queue close-on-panic guard.
pub fn run_directory_worker<W: DirectoryWorker>(
    queue: &WorkQueue<DirectoryTask<W::Id, W::Context>>,
    worker: &mut W,
) {
    while let Some(task) = queue.pop() {
        let listing = worker.list(&task);
        let prefetch = worker.prefetch(&task, &listing);
        let plan = task.measurement_plan(&listing.children, prefetch);
        let measuring = plan.is_some();
        if !worker.publish(DirectoryResult::Listed {
            id: task.id.clone(),
            listing,
            measuring,
        }) {
            return;
        }
        if let Some(plan) = plan {
            let measured = worker.measure(plan);
            if !worker.publish(DirectoryResult::Measured(measured)) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        DirectoryListing, DirectoryMeasurement, DirectoryMeasurementPlan, DirectoryResult,
        DirectoryTask, DirectoryWorker, MeasuredSize, WorkQueue, run_directory_worker,
    };
    use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

    struct Adapter<'a> {
        queue: &'a WorkQueue<DirectoryTask<String, Rc<()>>>,
        log: &'a RefCell<Vec<&'static str>>,
        results: Vec<DirectoryResult<String, Rc<()>>>,
        prefetch: bool,
        reject: Option<&'static str>,
        cancel_on_listing: bool,
        cancelled: bool,
        remaining: usize,
    }
    impl DirectoryWorker for Adapter<'_> {
        type Id = String;
        type Context = Rc<()>;
        fn list(&mut self, task: &DirectoryTask<String, Rc<()>>) -> DirectoryListing<Rc<()>> {
            self.log.borrow_mut().push("list");
            DirectoryListing {
                children: vec![(
                    PathBuf::from("Cargo.toml"),
                    fs::symlink_metadata("Cargo.toml")
                        .unwrap()
                        .file_type()
                        .into(),
                )],
                errors: vec![],
                context: task.context.clone(),
            }
        }
        fn prefetch(
            &mut self,
            _: &DirectoryTask<String, Rc<()>>,
            _: &DirectoryListing<Rc<()>>,
        ) -> bool {
            self.log.borrow_mut().push("prefetch");
            self.prefetch
        }
        fn measure(
            &mut self,
            plan: DirectoryMeasurementPlan<'_, String>,
        ) -> DirectoryMeasurement<String> {
            self.log.borrow_mut().push("measure");
            plan.measure(
                |path| fs::symlink_metadata(path),
                |_, _| {
                    |_: &std::path::Path| {
                        Ok(MeasuredSize {
                            apparent: 7,
                            allocated: 512,
                        })
                    }
                },
                || self.cancelled,
            )
        }
        fn publish(&mut self, result: DirectoryResult<String, Rc<()>>) -> bool {
            let (event, finished) = match &result {
                DirectoryResult::Listed { measuring, .. } => {
                    self.cancelled = self.cancel_on_listing;
                    ("listed", !measuring)
                }
                DirectoryResult::Measured(_) => ("measured", true),
            };
            self.log.borrow_mut().push(event);
            self.results.push(result);
            if self.reject == Some(event) {
                return false;
            }
            if finished {
                self.remaining -= 1;
                if self.remaining == 0 {
                    self.queue.close();
                }
            }
            true
        }
    }
    fn task(id: &str, measure: bool) -> DirectoryTask<String, Rc<()>> {
        DirectoryTask {
            id: id.into(),
            path: PathBuf::from("."),
            measure,
            context: Rc::new(()),
        }
    }
    fn adapter<'a>(
        queue: &'a WorkQueue<DirectoryTask<String, Rc<()>>>,
        log: &'a RefCell<Vec<&'static str>>,
    ) -> Adapter<'a> {
        Adapter {
            queue,
            log,
            results: vec![],
            prefetch: false,
            reject: None,
            cancel_on_listing: false,
            cancelled: false,
            remaining: 1,
        }
    }
    #[test]
    fn host_intent_and_prefetch_publish_listing_before_measurement() {
        for (intent, prefetch) in [(true, false), (false, true), (true, true)] {
            let queue = WorkQueue::default();
            let task = task("owned id", intent);
            let context = task.context.clone();
            queue.push(task).unwrap();
            let log = RefCell::new(vec![]);
            let mut host = adapter(&queue, &log);
            host.prefetch = prefetch;
            run_directory_worker(&queue, &mut host);
            assert_eq!(
                *log.borrow(),
                ["list", "prefetch", "listed", "measure", "measured"]
            );
            assert!(
                matches!(&host.results[0], DirectoryResult::Listed { id, listing, measuring: true } if id=="owned id" && Rc::ptr_eq(&listing.context,&context))
            );
            assert!(
                matches!(&host.results[1], DirectoryResult::Measured(m) if m.id=="owned id" && m.files.len()==1)
            );
        }
    }
    #[test]
    fn listing_only_does_not_measure() {
        let queue = WorkQueue::default();
        queue.push(task("first", false)).unwrap();
        let log = RefCell::new(vec![]);
        let mut host = adapter(&queue, &log);
        run_directory_worker(&queue, &mut host);
        assert_eq!(*log.borrow(), ["list", "prefetch", "listed"]);
        assert!(matches!(
            &host.results[0],
            DirectoryResult::Listed {
                measuring: false,
                ..
            }
        ));
    }
    #[test]
    fn rejected_publication_stops_without_popping_another_task() {
        for rejected in ["listed", "measured"] {
            let queue = WorkQueue::default();
            queue.push(task("untouched", true)).unwrap();
            queue.push(task("first", true)).unwrap();
            let log = RefCell::new(vec![]);
            let mut host = adapter(&queue, &log);
            host.reject = Some(rejected);
            run_directory_worker(&queue, &mut host);
            assert_eq!(queue.pop().unwrap().id, "untouched");
            queue.close();
            let expected = if rejected == "listed" {
                vec!["list", "prefetch", "listed"]
            } else {
                vec!["list", "prefetch", "listed", "measure", "measured"]
            };
            assert_eq!(*log.borrow(), expected);
        }
    }
    #[test]
    fn cancellation_after_listing_still_publishes_partial_measurement() {
        let queue = WorkQueue::default();
        queue.push(task("first", true)).unwrap();
        let log = RefCell::new(vec![]);
        let mut host = adapter(&queue, &log);
        host.cancel_on_listing = true;
        run_directory_worker(&queue, &mut host);
        assert_eq!(
            *log.borrow(),
            ["list", "prefetch", "listed", "measure", "measured"]
        );
        assert!(
            matches!(&host.results[1], DirectoryResult::Measured(m) if m.directory.is_ok() && m.files.is_empty())
        );
    }
    #[test]
    fn closed_queue_does_not_invoke_host() {
        let queue = WorkQueue::default();
        queue.close();
        let log = RefCell::new(vec![]);
        run_directory_worker(&queue, &mut adapter(&queue, &log));
        assert!(log.borrow().is_empty());
    }
    #[test]
    fn successful_publications_continue_in_queue_order() {
        let queue = WorkQueue::default();
        queue.push(task("second", true)).unwrap();
        queue.push(task("first", true)).unwrap();
        let log = RefCell::new(vec![]);
        let mut host = adapter(&queue, &log);
        host.remaining = 2;
        run_directory_worker(&queue, &mut host);
        let ids: Vec<_> = host
            .results
            .iter()
            .map(|r| match r {
                DirectoryResult::Listed { id, .. } => id.as_str(),
                DirectoryResult::Measured(m) => m.id.as_str(),
            })
            .collect();
        assert_eq!(ids, ["first", "first", "second", "second"]);
        assert_eq!(
            *log.borrow(),
            ["list", "prefetch", "listed", "measure", "measured"].repeat(2)
        );
    }
    #[test]
    fn host_panic_propagates_through_shutdown_guard() {
        let queue = WorkQueue::default();
        queue.push(task("first", true)).unwrap();
        let log = RefCell::new(vec![]);
        let mut host = adapter(&queue, &log);
        // Keep the same host shutdown obligation as scoped worker assembly.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _shutdown = queue.close_on_drop();
            let _borrow = log.borrow_mut();
            run_directory_worker(&queue, &mut host);
        }));
        assert!(result.is_err());
        assert!(queue.pop().is_none());
        assert!(host.results.is_empty());
    }
}
