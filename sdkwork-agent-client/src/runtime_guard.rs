use std::sync::{Mutex, MutexGuard};

/// Lock a plugin runtime mutex without panicking on poison.
pub fn lock_runtime_mutex<'a, T>(mutex: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, String> {
    mutex
        .lock()
        .map_err(|error| format!("lock poisoned: {error}"))
}
