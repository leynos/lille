use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

static VERBOSE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn init(verbose: bool) {
    VERBOSE.store(verbose, Ordering::SeqCst);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if $crate::logging::is_verbose() {
            eprintln!($($arg)*);
        }
    };
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}