//! Best-effort propagation of the caller's requested QoS to scanner workers.

#[cfg(target_os = "macos")]
use std::ffi::c_void;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pthread_self() -> *mut c_void;
    fn pthread_get_qos_class_np(
        thread: *mut c_void,
        qos_class: *mut u32,
        relative_priority: *mut i32,
    ) -> i32;
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
}

#[cfg(target_os = "macos")]
const QOS_CLASS_UNSPECIFIED: u32 = 0x00;

/// QoS requested by the scanner's calling thread, if Darwin reports one.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RequestedQos(Option<(u32, i32)>);

#[cfg(target_os = "macos")]
fn from_requested_values(qos_class: u32, relative_priority: i32) -> RequestedQos {
    if qos_class == QOS_CLASS_UNSPECIFIED {
        RequestedQos(None)
    } else {
        RequestedQos(Some((qos_class, relative_priority)))
    }
}

/// Portable placeholder: other platforms retain their native thread behavior.
#[cfg(not(target_os = "macos"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RequestedQos;

#[cfg(target_os = "macos")]
pub(super) fn capture() -> RequestedQos {
    let mut qos_class = QOS_CLASS_UNSPECIFIED;
    let mut relative_priority = 0;
    // SAFETY: pthread_self returns the current thread, and both output pointers
    // refer to initialized writable values for the duration of the call.
    let result =
        unsafe { pthread_get_qos_class_np(pthread_self(), &mut qos_class, &mut relative_priority) };

    if result == 0 {
        from_requested_values(qos_class, relative_priority)
    } else {
        RequestedQos(None)
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn capture() -> RequestedQos {
    RequestedQos
}

#[cfg(target_os = "macos")]
pub(super) fn apply(requested: RequestedQos) {
    if let Some((qos_class, relative_priority)) = requested.0 {
        // SAFETY: the values came from pthread_get_qos_class_np on this
        // process. Failure is best-effort and must not fail the scan.
        let _ = unsafe { pthread_set_qos_class_self_np(qos_class, relative_priority) };
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn apply(_requested: RequestedQos) {}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    const QOS_CLASS_DEFAULT: u32 = 0x15;
    const QOS_CLASS_USER_INITIATED: u32 = 0x19;

    fn current_qos() -> (u32, i32) {
        let mut qos_class = QOS_CLASS_UNSPECIFIED;
        let mut relative_priority = 0;
        // SAFETY: pthread_self identifies this test thread and both pointers
        // refer to writable output values.
        let result = unsafe {
            pthread_get_qos_class_np(pthread_self(), &mut qos_class, &mut relative_priority)
        };
        assert_eq!(result, 0);
        (qos_class, relative_priority)
    }

    fn set_current_qos(qos_class: u32, relative_priority: i32) {
        // SAFETY: tests pass QoS classes and relative priorities declared by
        // the Darwin API, and each test confines the change to its own thread.
        let result = unsafe { pthread_set_qos_class_self_np(qos_class, relative_priority) };
        assert_eq!(result, 0);
    }

    #[test]
    fn propagates_user_initiated_class_and_relative_priority() {
        let requested = std::thread::spawn(|| {
            set_current_qos(QOS_CLASS_USER_INITIATED, -4);
            let requested = capture();
            assert_eq!(requested.0, Some((QOS_CLASS_USER_INITIATED, -4)));
            requested
        })
        .join()
        .expect("QoS capture thread panicked");

        let applied = std::thread::spawn(move || {
            apply(requested);
            current_qos()
        })
        .join()
        .expect("QoS apply thread panicked");

        assert_eq!(applied, (QOS_CLASS_USER_INITIATED, -4));
    }

    #[test]
    fn propagates_default_class_and_relative_priority() {
        let requested = std::thread::spawn(|| {
            set_current_qos(QOS_CLASS_DEFAULT, -2);
            let requested = capture();
            assert_eq!(requested.0, Some((QOS_CLASS_DEFAULT, -2)));
            requested
        })
        .join()
        .expect("default QoS capture thread panicked");

        let applied = std::thread::spawn(move || {
            apply(requested);
            current_qos()
        })
        .join()
        .expect("default QoS apply thread panicked");

        assert_eq!(applied, (QOS_CLASS_DEFAULT, -2));
    }

    #[test]
    fn unspecified_qos_is_a_no_op() {
        assert_eq!(
            from_requested_values(QOS_CLASS_UNSPECIFIED, 0),
            RequestedQos(None)
        );
        std::thread::spawn(|| {
            set_current_qos(QOS_CLASS_USER_INITIATED, -3);
            let before = current_qos();
            apply(RequestedQos(None));
            assert_eq!(current_qos(), before);
        })
        .join()
        .expect("unspecified QoS test thread panicked");
    }
}
