//! Thread policy lifetime for non-materializing filesystem I/O.
//! Hosts choose where to apply this guard. It is not a universal provider guarantee.
//! Restoration is best effort, as in the original Darwin implementation.

#[cfg(target_os = "macos")]
mod darwin {
    use std::{io, marker::PhantomData, rc::Rc};

    // Public Darwin constants from <sys/resource.h>.
    const MATERIALIZE_DATALESS_FILES: i32 = 3;
    const THREAD: i32 = 1;
    pub(super) const OFF: i32 = 1;

    unsafe extern "C" {
        fn getiopolicy_np(iotype: i32, scope: i32) -> i32;
        fn setiopolicy_np(iotype: i32, scope: i32, policy: i32) -> i32;
    }

    pub(super) fn current() -> io::Result<i32> {
        // SAFETY: these are documented Darwin type/scope constants; no pointers.
        let policy = unsafe { getiopolicy_np(MATERIALIZE_DATALESS_FILES, THREAD) };
        if policy == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(policy)
        }
    }

    fn set(policy: i32) -> io::Result<()> {
        // SAFETY: policy is OFF or a value obtained from getiopolicy_np.
        if unsafe { setiopolicy_np(MATERIALIZE_DATALESS_FILES, THREAD, policy) } == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Must be dropped on the thread whose policy it changed, including unwind.
    pub struct NonMaterializingIoGuard {
        previous: i32,
        _same_thread: PhantomData<Rc<()>>,
    }

    impl NonMaterializingIoGuard {
        /// Read the calling thread's raw Darwin policy value for diagnostics.
        pub fn current_policy() -> io::Result<i32> {
            current()
        }

        pub fn enter() -> io::Result<Self> {
            let previous = current()?;
            set(OFF)?;
            Ok(Self {
                previous,
                _same_thread: PhantomData,
            })
        }
    }

    impl Drop for NonMaterializingIoGuard {
        fn drop(&mut self) {
            // This valid policy was read on this same thread. Never panic in Drop.
            let _ = set(self.previous);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn guard_restores_policy_after_nested_use_and_unwind() {
            let before = current().unwrap();
            let result = std::panic::catch_unwind(|| {
                let _outer = NonMaterializingIoGuard::enter().unwrap();
                assert_eq!(current().unwrap(), OFF);
                {
                    let _inner = NonMaterializingIoGuard::enter().unwrap();
                    assert_eq!(current().unwrap(), OFF);
                }
                assert_eq!(current().unwrap(), OFF);
                panic!("exercise policy restoration");
            });
            assert!(result.is_err());
            assert_eq!(current().unwrap(), before);
        }
    }
}

#[cfg(target_os = "macos")]
pub use darwin::NonMaterializingIoGuard;

/// No-op on non-macOS targets; makes no provider/download-prevention claim.
#[cfg(not(target_os = "macos"))]
pub struct NonMaterializingIoGuard;
#[cfg(not(target_os = "macos"))]
impl NonMaterializingIoGuard {
    pub fn enter() -> std::io::Result<Self> {
        Ok(Self)
    }
}
