//! Locks that survive a panic. A lock held by a thread that panics is poisoned, and `lock().unwrap()` on it panics
//! too, so one fault in one place would end every command and every event after it. Here a poisoned lock is taken
//! all the same, with what it holds as it was left: a view a moment out of date is better than no Kuchu.

use std::sync::{Mutex, MutexGuard, PoisonError};

pub trait Hold<T> {
    /// Takes the lock, poisoned or not.
    fn hold(&self) -> MutexGuard<'_, T>;
}

impl<T> Hold<T> for Mutex<T> {
    fn hold(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_poisoned_by_a_panic_is_still_taken() {
        let counter = std::sync::Arc::new(Mutex::new(1));
        let poisoner = counter.clone();
        let _ = std::thread::spawn(move || {
            let mut held = poisoner.hold();
            *held += 1;
            panic!("while holding the lock");
        })
        .join();
        assert!(counter.is_poisoned());
        assert_eq!(*counter.hold(), 2);
    }
}
