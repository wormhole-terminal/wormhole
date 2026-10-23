/// Drop the background policy a host started by the app might inherit.
#[allow(unsafe_code)] // libc FFI; constant arguments.
pub(crate) fn clear_background_policy() {
    // SAFETY: setpriority takes plain values.
    unsafe { libc::setpriority(libc::PRIO_DARWIN_PROCESS, 0, 0) };
}

/// Put the calling thread at the user-interactive QoS class.
#[allow(unsafe_code)] // libc FFI; constant arguments.
pub(crate) fn set_user_interactive() {
    // SAFETY: changes only the calling thread's QoS class.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}
