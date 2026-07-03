use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn install_shutdown_handlers() {
    #[cfg(unix)]
    unsafe {
        unix_signal::install();
    }
}

pub(crate) fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Relaxed)
}

extern "C" fn request_shutdown(_: i32) {
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
mod unix_signal {
    unsafe extern "C" {
        fn signal(signum: i32, handler: extern "C" fn(i32)) -> extern "C" fn(i32);
    }

    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;

    pub(crate) unsafe fn install() {
        signal(SIGINT, super::request_shutdown);
        signal(SIGTERM, super::request_shutdown);
    }
}
