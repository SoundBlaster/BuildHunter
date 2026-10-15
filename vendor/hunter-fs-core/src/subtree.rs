//! Byte aggregation and host-announced subtree completion; no filesystem or policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtreeCompletion {
    pub bytes: u64,
    pub partial: bool,
}

/// State for one host-selected subtree. The host supplies all membership/measurement
/// decisions and announces every pending descendant directory exactly once.
/// Contains no identity, paths, hierarchy, queues, callbacks or heap allocation.
#[derive(Debug, Default)]
pub struct SubtreeProgress {
    bytes: u64,
    pending: usize,
    partial: bool,
    root_finished: bool,
    completed: bool,
}
impl SubtreeProgress {
    /// Units are host-supplied bytes; addition saturates and does not deduplicate.
    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_add(bytes);
    }
    /// Replace a known leaf's own total rather than adding it to previous accounting.
    pub fn set_bytes(&mut self, bytes: u64) {
        self.bytes = bytes;
    }
    pub fn mark_partial(&mut self) {
        self.partial = true;
    }

    /// Record one descendant directory before host dispatch, without scheduling it.
    pub fn add_pending_directory(&mut self) {
        self.pending = self
            .pending
            .checked_add(1)
            .expect("pending directory count fits usize");
    }
    /// Retire one announced descendant directory. Panics if none was outstanding.
    pub fn finish_pending_directory(&mut self) {
        self.pending = self
            .pending
            .checked_sub(1)
            .expect("directory completion requires pending work");
    }
    /// Host has processed the selected root's own measurements/children.
    pub fn finish_root(&mut self) {
        self.root_finished = true;
    }
    pub fn is_completed(&self) -> bool {
        self.completed
    }
    /// Readiness reflects host announcements, not actual worker completion or scope join.
    pub fn is_ready(&self) -> bool {
        self.root_finished && self.pending == 0 && !self.completed
    }

    /// Finalize once on host instruction. A later call returns None. Normal directory
    /// finalization requires the host to check is_ready; known leaves may finalize
    /// directly and early stop may force a partial prefix while directories remain.
    /// This method does not enforce readiness or cancel/join work. Byte/counter updates
    /// remain caller-owned after finalization; no second completion can be emitted.
    pub fn complete(&mut self, force_partial: bool) -> Option<SubtreeCompletion> {
        if self.completed {
            return None;
        }
        self.completed = true;
        Some(SubtreeCompletion {
            bytes: self.bytes,
            partial: self.partial || force_partial,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_root_becomes_ready_and_completes_once() {
        let mut state = SubtreeProgress::default();
        assert!(!state.is_ready());
        assert!(!state.is_completed());
        state.finish_root();
        assert!(state.is_ready());
        assert_eq!(
            state.complete(false),
            Some(SubtreeCompletion {
                bytes: 0,
                partial: false
            })
        );
        assert!(state.is_completed());
        assert!(!state.is_ready());
        assert_eq!(state.complete(false), None);
    }
    #[test]
    fn finished_root_waits_for_every_announced_directory() {
        let mut state = SubtreeProgress::default();
        state.add_pending_directory();
        state.add_pending_directory();
        state.finish_root();
        assert!(!state.is_ready());
        state.add_bytes(10);
        state.finish_pending_directory();
        assert!(!state.is_ready());
        state.add_bytes(20);
        state.finish_pending_directory();
        assert!(state.is_ready());
        assert_eq!(state.complete(false).unwrap().bytes, 30);
    }
    #[test]
    fn nested_states_follow_host_accounting_and_wait_for_their_own_root() {
        let mut outer = SubtreeProgress::default();
        let mut inner = SubtreeProgress::default();
        outer.add_pending_directory(); // inner root
        outer.add_pending_directory();
        inner.add_pending_directory(); // deeper directory
        outer.add_bytes(7);
        inner.add_bytes(7);
        outer.finish_pending_directory();
        inner.finish_pending_directory();
        assert!(!inner.is_ready());
        inner.finish_root();
        assert!(inner.is_ready());
        assert_eq!(inner.complete(false).unwrap().bytes, 7);
        outer.finish_pending_directory();
        assert!(!outer.is_ready());
        outer.add_bytes(5);
        outer.finish_root();
        assert!(outer.is_ready());
        assert_eq!(outer.complete(false).unwrap().bytes, 12);
    }
    #[test]
    fn byte_addition_saturates_instead_of_wrapping() {
        let mut state = SubtreeProgress::default();
        state.add_bytes(u64::MAX - 1);
        state.add_bytes(100);
        assert_eq!(state.complete(false).unwrap().bytes, u64::MAX);
    }
    #[test]
    fn known_leaf_size_replaces_the_total() {
        let mut state = SubtreeProgress::default();
        state.add_bytes(4);
        state.set_bytes(9);
        assert_eq!(
            state.complete(false),
            Some(SubtreeCompletion {
                bytes: 9,
                partial: false
            })
        );
    }
    #[test]
    fn partial_marker_and_host_force_combine_without_losing_bytes() {
        for marked in [false, true] {
            for forced in [false, true] {
                let mut state = SubtreeProgress::default();
                state.add_bytes(17);
                if marked {
                    state.mark_partial();
                }
                assert_eq!(
                    state.complete(forced),
                    Some(SubtreeCompletion {
                        bytes: 17,
                        partial: marked || forced
                    })
                );
            }
        }
    }
    #[test]
    fn host_can_force_a_partial_prefix_while_directories_are_pending() {
        let mut state = SubtreeProgress::default();
        state.add_pending_directory();
        state.add_bytes(23);
        assert_eq!(
            state.complete(true),
            Some(SubtreeCompletion {
                bytes: 23,
                partial: true
            })
        );
        state.finish_pending_directory();
        state.finish_root();
        assert!(!state.is_ready());
        assert!(state.is_completed());
        assert_eq!(state.complete(false), None);
    }
    #[test]
    #[should_panic(expected = "directory completion requires pending work")]
    fn unannounced_directory_completion_is_a_host_protocol_error() {
        SubtreeProgress::default().finish_pending_directory();
    }
}
