extern crate crossbeam;

use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam::epoch::{self, Atomic, Owned, Guard};

pub struct LazyTransform<T, S, FN> {
    transform_fn: FN,
    source: Atomic<S>,
    value: Atomic<T>,
    transform_lock: LightLock,
}

impl<T: Clone, S, FN: Fn(S) -> Option<T>> LazyTransform<T, S, FN> {
    pub fn new(transform_fn: FN) -> LazyTransform<T, S, FN> {
        LazyTransform {
            transform_fn: transform_fn,
            source: Atomic::null(),
            value: Atomic::null(),
            transform_lock: LightLock::new(),
        }
    }

    pub fn set_source(&self, source: S) {
        let guard = epoch::pin();
        let prev = self.source.swap(Some(Owned::new(source)),
                                  Ordering::AcqRel, &guard);
        if let Some(prev) = prev {
            unsafe { guard.unlinked(prev); }
        }
    }

    fn try_transform(&self, guard: &Guard) -> Option<T> {
        if let Some(_lock_guard) = self.transform_lock.try_lock() {
            let source_maybe = self.source.swap(None, Ordering::AcqRel, &guard);
            let source = match source_maybe {
                Some(source) => source,
                None => return None,
            };
            let source_data = unsafe { ::std::ptr::read(source.as_raw()) };
            let newval = match (self.transform_fn)(source_data) {
                Some(newval) => newval,
                None => return None,
            };
            let prev = self.value.swap(Some(Owned::new(newval.clone())),
                                       Ordering::AcqRel, &guard);
            unsafe {
                if let Some(prev) = prev {
                    guard.unlinked(prev);
                }
                guard.unlinked(source);
            }
            return Some(newval);
        }
        None
    }

    pub fn get_transformed(&self) -> Option<T> {
        let guard = epoch::pin();
        let source = self.source.load(Ordering::Relaxed, &guard);
        if source.is_some() {
            let newval = self.try_transform(&guard);
            if newval.is_some() {
                return newval;
            }
        }
        self.value.load(Ordering::Acquire, &guard)
            .as_ref().map(|x| T::clone(&x))
    }
}


#[derive(Debug)]
struct LightLock(AtomicBool);

impl LightLock {
    pub fn new() -> LightLock {
        LightLock(AtomicBool::new(false))
    }

    pub fn try_lock<'a>(&'a self) -> Option<LightGuard<'a>> {
        let was_locked = self.0.swap(true, Ordering::Acquire);
        if was_locked {
            None
        } else {
            Some(LightGuard { lock: self })
        }
    }
}

struct LightGuard<'a> {
    lock: &'a LightLock,
}

impl<'a> Drop for LightGuard<'a> {
    fn drop(&mut self) {
        self.lock.0.store(false, Ordering::Release);
    }
}

fn main() {
    use std::sync::Arc;

    fn to_ns(x: ::std::time::Duration, iters: usize) -> f64 {
        let ns = x.as_secs() * 1_000_000_000 + x.subsec_nanos() as u64;
        ns as f64 / iters as f64
    }

    let lt = Arc::new(LazyTransform::new(|x: u64| Some(x + 1)));
    lt.set_source(123);
    //lt.force();

    const ITERS: usize = 1_000_000;

    let threads = (0..8).map(|i| {
        std::thread::spawn({
            let lt = Arc::clone(&lt);
            move || {
                let t0 = ::std::time::Instant::now();
                for _ in 0..ITERS {
                    assert!(lt.get_transformed().map(|x| x) == Some(124));
                }
                let t1 = ::std::time::Instant::now();
                println!("Consumer-{}: {}", i, to_ns(t1 - t0, ITERS));
            }
        })
    }).collect::<Vec<_>>();
    for t in threads {
        t.join().unwrap();
    }
}
