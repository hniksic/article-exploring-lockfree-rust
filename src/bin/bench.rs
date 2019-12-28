extern crate lazy_transform;
extern crate crossbeam;
extern crate time;
extern crate rand;

use rand::Rng;

use lazy_transform::LazyTransform;

#[derive(Debug, Clone)]
struct Payload(String);

fn parse_bytes(b: Box<[u8]>) -> Option<Payload> {
    let start = time::precise_time_ns();
    let mut d = time::precise_time_ns() as f64 / start as f64;
    for _ in 0..10_000 {
        d *= 1.00001;
    }

    Some(Payload(String::from_utf8(b.into_vec()).unwrap()
                 + &format!("{}", d)))
}

type BenchLazyTransform<FN> = LazyTransform<Payload, Box<[u8]>, FN>;

const PRODUCE_ITERS: usize = 1_000_000;
const CONSUME_ITERS: usize = 100_000_000;

fn produce<FN>(lt: &BenchLazyTransform<FN>)
    where FN: Fn(Box<[u8]>) -> Option<Payload>
{
    fn random_byte() -> u8 {
        'A' as u8 + rand::thread_rng().gen_range(0u8, 10)
    }

    let start = time::precise_time_ns();
    for _i in 0..PRODUCE_ITERS {
        lt.set_source((0..3).map(|_| random_byte())
                      .collect::<Vec<_>>().into_boxed_slice());
        simulate_work();
    }
    let elapsed = time::precise_time_ns() - start;
    println!("Producer took {} ns/op ({} s)",
             elapsed as f64 / PRODUCE_ITERS as f64,
             elapsed as f64 / 1e9);
}

fn consume<FN>(lt: &BenchLazyTransform<FN>)
    where FN: Fn(Box<[u8]>) -> Option<Payload>
{
    let start = time::precise_time_ns();
    let mut count = 0u64;
    for _ in 0..CONSUME_ITERS {
        if let Some(o) = lt.get_transformed() {
            if o.0 == "longer" {
                count += 1;
            }
        }
    }
    let elapsed = time::precise_time_ns() - start;
    println!("Consumer took {} ns/op ({} s, count {})",
             elapsed as f64 / CONSUME_ITERS as f64,
             elapsed as f64 / 1e9, count);
}

fn simulate_work() {
    static mut BLACK_HOLE: f64 = 0f64;

    let start = time::precise_time_ns();
    let mut d = time::precise_time_ns() as f64 / start as f64;
    for _ in 0..10_000 {
        d *= 1.00001;
    }
    unsafe {
        BLACK_HOLE = if d > 1.1 { d } else { 2.0 * d };
    }
}

fn main() {
    let lt = LazyTransform::new(parse_bytes);
    for _ in 0..3 {
        crossbeam::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| consume(&lt));
            }
            println!("Start producing");
            produce(&lt);
        });
    }
}
