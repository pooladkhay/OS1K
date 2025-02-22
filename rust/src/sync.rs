use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

#[repr(u8)]
enum OnceState {
    Incomplete = 0,
    InProgress = 1,
    Completed = 2,
}

/// A minimal implementation of `Once`.
///
/// This type guarantees that the provided closure is executed only once.
/// It uses an atomic state with three values: `Incomplete`, `InProgress` and `Completed`.
///
/// If the closure panics, the Once state remains InProgress, causing subsequent calls to spin indefinitely.
///
struct Once {
    state: AtomicU8,
}

impl Once {
    /// Creates a new `Once` instance.
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(OnceState::Incomplete as u8),
        }
    }

    /// Calls the provided closure only once.
    ///
    /// If the closure is already running in another execution context,
    /// this method spins (using `core::hint::spin_loop()`) until execution is completed.
    fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.is_complete() {
            return;
        }

        if self
            .state
            .compare_exchange(
                OnceState::Incomplete as u8,
                OnceState::InProgress as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            f();

            self.state
                .store(OnceState::Completed as u8, Ordering::Release);
        } else {
            while !self.is_complete() {
                spin_loop();
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.state.load(Ordering::Acquire) == OnceState::Completed as u8
    }
}

/// A minimal `OnceCell`` for safe one‑time initialization of a value.
///
/// It encapsulates an `Option<T>` inside an `UnsafeCell` and uses `Once` to ensure
/// that initialization (via the provided closure) happens only once.
pub struct OnceCell<T> {
    once: Once,
    value: UnsafeCell<Option<T>>,
}

// Safety: It is safe to share OnceCell between threads if T is Sync.
unsafe impl<T: Sync> Sync for OnceCell<T> {}

// Safety: OnceCell can be sent to another thread if T is Send.
unsafe impl<T: Send> Send for OnceCell<T> {}

impl<T> OnceCell<T> {
    /// Creates a new, uninitialized `OnceCell`.
    pub const fn new() -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(None),
        }
    }

    /// Returns an immutable reference to the stored value,
    /// initializing it with the provided closure if it hasn't been already.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        self.once.call_once(|| {
            // Safety: We have exclusive initialization through `Once`.
            unsafe {
                *self.value.get() = Some(f());
            }
        });
        // Safety: Initialization is complete, so the value is guaranteed to be Some.
        unsafe { (*self.value.get()).as_ref().unwrap() }
    }
}

#[repr(u8)]
enum MutexState {
    Free = 0,
    Locked = 1,
}

/// A spinlock-based (using `core::hint::spin_loop()`) mutex.
pub struct Mutex<T> {
    lock: AtomicU8,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex wrapping the supplied data.
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicU8::new(MutexState::Free as u8),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, spinning (using `core::hint::spin_loop()`) until it becomes available.
    pub fn lock(&self) -> MutexGuard<T> {
        while self
            .lock
            .compare_exchange(
                MutexState::Free as u8,
                MutexState::Locked as u8,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

/// A guard that releases the lock when dropped.
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safety: we hold the lock.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: we hold the lock.
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex
            .lock
            .store(MutexState::Free as u8, Ordering::Release);
    }
}

// Safety: Our simple mutex is safe to share between threads as long as T is Send.
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}
