use std::sync::atomic::{AtomicBool, Ordering};

use coco::epoch::{self, Atomic, Owned, Ptr, Scope};

#[derive(Debug)]
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

    // Publish a new source.
    pub fn set_source(&self, source: S) {
        epoch::pin(|scope| unsafe {
            let source_ptr = Owned::new(source).into_ptr(&scope);
            let prev = self.source.swap(source_ptr, Ordering::AcqRel, &scope);
            if !prev.is_null() {
                scope.defer_drop(prev);
            }
        });
    }

    // Transform and drop the newly published SOURCE if available.  Caches the
    // new value and returns a copy.  Returns None if no new source exists, if
    // the lock is already taken, or if transformation fails.
    fn try_transform(&self, scope: &Scope) -> Option<T> {
        if let Some(_lock_guard) = self.transform_lock.try_lock() {
            let source = self.source.swap(Ptr::null(), Ordering::AcqRel, &scope);
            if source.is_null() {
                return None;
            }
            let source_data;
            unsafe {
                source_data = ::std::ptr::read(source.as_raw());
                scope.defer_free(source);
            }
            let newval = match (self.transform_fn)(source_data) {
                Some(newval) => newval,
                None => return None,
            };
            let prev = self.value.swap(Owned::new(newval.clone()).into_ptr(&scope),
                                       Ordering::AcqRel, &scope);
            unsafe {
                if !prev.is_null() {
                    scope.defer_drop(prev);
                }
            }
            return Some(newval);
        }
        None
    }

    // Lazily generate a new value if a new source is provided.  Otherwise,
    // return the cached value.
    pub fn get_transformed(&self) -> Option<T> {
        epoch::pin(|scope| {
            let source = self.source.load(Ordering::Relaxed, &scope);
            if !source.is_null() {
                let newval = self.try_transform(&scope);
                if newval.is_some() {
                    return newval;
                }
            }
            unsafe {
                self.value.load(Ordering::Acquire, &scope)
                    .as_ref().map(T::clone)
            }
        })
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
