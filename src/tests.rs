use lazy_transform::LazyTransform;

use std::sync::Arc;
use std::thread;
use std::time::{Instant, Duration};

fn busy_wait(nsecs: u32) {
    let deadline = Instant::now() + Duration::new(0, nsecs);
    while Instant::now() < deadline {
    }
}

trait Behavior: Send + Sync {
    fn as_pair(&self) -> (u64, u64);
}

impl Behavior for (u64, u64) {
    fn as_pair(&self) -> (u64, u64) {
        (self.0, self.1)
    }
}

fn transform_to_opaque(s: String) -> Option<Arc<Behavior>> {
    let nums: Vec<_> = s.split_whitespace().collect();
    if nums.len() != 2 {
        return None;
    }
    busy_wait(3000);
    let num1 = nums[0].parse();
    let num2 = nums[1].parse();
    if num1.is_err() || num2.is_err() {
        return None;
    }
    Some(Arc::new((num1.unwrap(), num2.unwrap())))
}

#[test]
fn simple() {
    let lt = LazyTransform::new(transform_to_opaque);
    assert!(lt.get_transformed().is_none());
    lt.set_source("123 456".to_owned());
    assert_eq!(lt.get_transformed().unwrap().as_pair(), (123, 456));
    assert_eq!(lt.get_transformed().unwrap().as_pair(), (123, 456));
    lt.set_source("456 789".to_owned());
    assert_eq!(lt.get_transformed().unwrap().as_pair(), (456, 789));
}

#[test]
fn threaded() {
    let lt = Arc::new(LazyTransform::new(transform_to_opaque));
    thread::spawn({
        let lt = Arc::clone(&lt);
        move || lt.set_source("12 3".to_owned())
    }).join().unwrap();
    assert_eq!(lt.get_transformed().unwrap().as_pair(), (12, 3));
}

fn transform_to_concrete(s: String) -> Option<u64> {
    s.parse().ok()
}

#[test]
fn heavy() {
    let t0 = ::std::time::Instant::now();
    let lt = Arc::new(LazyTransform::new(transform_to_concrete));
    const ITERS: u64 = 1_000_000;
    let producer = thread::spawn({
        let lt = Arc::clone(&lt);
        move || {
            for i in 0..ITERS {
                lt.set_source(format!("{}", i));
                busy_wait(10);
            }
        }
    });
    let consumers: Vec<_> = (0..16).map(|_| thread::spawn({
        let lt = Arc::clone(&lt);
        move || {
            let mut cnt = 0u64;
            let mut last = None;
            while {
                let this = lt.get_transformed();
                match (last, this) {
                    (Some(last), Some(this)) => assert!(this >= last),
                    (Some(_), None) => panic!("Some followed by None"),
                    _ => ()
                }
                last = this;
                this
            } != Some(ITERS - 1) {
                cnt += 1;
                if cnt > 100 * ITERS {
                    panic!("{:?}", last);
                }
            }
            cnt
        }
    })).collect();
    producer.join().unwrap();
    let counts: u64 = consumers.into_iter()
        .map(|consumer| consumer.join().unwrap())
        .sum();
    let t1 = ::std::time::Instant::now();
    println!("done 2 {:?} {}", t1-t0, counts);
}

#[test]
fn heavy_arc() {
    let t0 = ::std::time::Instant::now();
    let lt = Arc::new(LazyTransform::new(transform_to_opaque));
    const ITERS: u64 = 1_000;
    let producer = thread::spawn({
        let lt = Arc::clone(&lt);
        move || {
            for i in 0..ITERS {
                for j in 0..ITERS {
                    lt.set_source(format!("{} {}", i, j));
                }
            }
        }
    });
    let consumers: Vec<_> = (0..16).map(|_| thread::spawn({
        let lt = Arc::clone(&lt);
        move || {
            let mut last;
            let mut cnt = 0;
            while {
                let this = lt.get_transformed();
                let end = this.as_ref().map(|x| x.as_pair()) != Some((ITERS - 1, ITERS - 1));
                last = this;
                end
            } {
                cnt += 1;
                if cnt > 10 * ITERS * ITERS {
                    panic!("heavy {:?}", last.as_ref().map(|l| l.as_pair()));
                }
            }
        }
    })).collect();
    producer.join().unwrap();
    for consumer in consumers {
        consumer.join().unwrap();
    }
    let t1 = ::std::time::Instant::now();
    println!("heavy done 2 {:?}", t1-t0);
}
