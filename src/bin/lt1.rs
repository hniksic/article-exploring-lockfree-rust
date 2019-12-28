use std::sync::Mutex;

pub struct LazyTransform<T, S, FN> {
    transform_fn: FN,
    source: Mutex<Option<S>>,
    value: Mutex<Option<T>>,
}

impl<T: Clone, S, FN: Fn(S) -> Option<T>> LazyTransform<T, S, FN> {
    pub fn new(transform_fn: FN) -> LazyTransform<T, S, FN> {
        LazyTransform {
            transform_fn: transform_fn,
            source: Mutex::new(None), value: Mutex::new(None),
        }
    }

    pub fn set_source(&self, source: S) {
        let mut locked_source = self.source.lock().unwrap();
        *locked_source = Some(source);
    }

    pub fn get_transformed(&self) -> Option<T> {
        let mut locked_value = self.value.lock().unwrap();
        let new_source = {
            let mut locked_source = self.source.lock().unwrap();
            locked_source.take()
        };
        if let Some(new_source) = new_source {
            let new_value = (self.transform_fn)(new_source);
            if new_value.is_some() {
                *locked_value = new_value;
            }
        }
        locked_value.clone()
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

    const ITERS: usize = 1_000_000;

    let threads = (0..8).map(|i| {
        std::thread::spawn({
            let lt = Arc::clone(&lt);
            move || {
                let t0 = ::std::time::Instant::now();
                for _ in 0..ITERS {
                    assert!(lt.get_transformed() == Some(124));
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
