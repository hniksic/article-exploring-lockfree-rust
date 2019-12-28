use std::sync::Mutex;

struct LazyState<T, S> {
    source: Option<S>,
    value: Option<T>,
}

pub struct LazyTransform<T, S, FN> {
    transform_fn: FN,
    state: Mutex<LazyState<T, S>>,
}

impl<T: Clone, S, FN: Fn(S) -> Option<T>> LazyTransform<T, S, FN> {
    pub fn new(transform_fn: FN) -> LazyTransform<T, S, FN> {
        LazyTransform {
            transform_fn: transform_fn,
            state: Mutex::new(LazyState { source: None, value: None }),
        }
    }

    pub fn set_source(&self, source: S) {
        let mut state = self.state.lock().unwrap();
        state.source = Some(source);
    }

    pub fn get_transformed(&self) -> Option<T> {
        let mut state = self.state.lock().unwrap();
        if let Some(new_source) = state.source.take() {
            let new_value = (self.transform_fn)(new_source);
            if new_value.is_some() {
                state.value = new_value;
            }
        }
        state.value.clone()
    }
}

fn main() {
    use std::sync::Arc;

    fn to_ns(x: ::std::time::Duration, iters: usize) -> f64 {
        let ns = x.as_secs() * 1_000_000_000 + x.subsec_nanos() as u64;
        ns as f64 / iters as f64
    }

    let lt = Arc::new(LazyTransform::new(|x: u64| Some(Arc::new(x + 1))));
    lt.set_source(123);

    const ITERS: usize = 1_000_000;

    let threads = (0..8).map(|i| {
        std::thread::spawn({
            let lt = Arc::clone(&lt);
            move || {
                let t0 = ::std::time::Instant::now();
                for _ in 0..ITERS {
                    assert!(lt.get_transformed().map(|x| *x) == Some(124));
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
