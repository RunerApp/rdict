use rdict::{LookupEntry, RdictReader, ReadMode};
use serde::Serialize;
use std::time::{Duration, Instant};

fn bench(path: &str, mode: ReadMode, label: &str) {
    let t0 = Instant::now();
    let mut r = RdictReader::open_with_mode(path, mode).expect("open");
    let open_time = t0.elapsed();
    println!("\n=== {} === open: {:?}", label, open_time);

    let words = [
        "abandon", "ability", "apple", "run", "the", "test", "hello", "world", "zebra", "cat",
    ];

    // Cold lookups (first hit on each block)
    let mut cold_times = Vec::new();
    for w in &words {
        let t = Instant::now();
        let _ = r.lookup(w);
        cold_times.push(t.elapsed());
    }
    println!("  cold lookups: {:?}", cold_times);
    println!(
        "  cold avg: {:.1} us",
        cold_times.iter().sum::<Duration>().as_micros() as f64 / cold_times.len() as f64
    );

    // Warm lookups (blocks now cached)
    for w in &words {
        let _ = r.lookup(w);
    }
    let t1 = Instant::now();
    for _ in 0..1000 {
        for w in &words {
            let _ = r.lookup(w);
        }
    }
    let e1 = t1.elapsed();
    println!(
        "  warm raw x10000: {:?} ({:.1} us/lookup)",
        e1,
        e1.as_micros() as f64 / 10000.0
    );

    // Warm lookup + clone
    let t2 = Instant::now();
    for _ in 0..1000 {
        for w in &words {
            if let Ok(Some(LookupEntry::Decoded(e))) = r.lookup(w) {
                let _ = (*e).clone();
            }
        }
    }
    let e2 = t2.elapsed();
    println!(
        "  warm clone x10000: {:?} ({:.1} us/lookup)",
        e2,
        e2.as_micros() as f64 / 10000.0
    );

    // Warm lookup + clone + json
    #[derive(Serialize)]
    struct Resp<'a> {
        ok: bool,
        found: bool,
        entry: Option<&'a rdict::Entry>,
    }
    let t3 = Instant::now();
    for _ in 0..1000 {
        for w in &words {
            if let Ok(Some(LookupEntry::Decoded(e))) = r.lookup(w) {
                let cloned = (*e).clone();
                let resp = Resp {
                    ok: true,
                    found: true,
                    entry: Some(&cloned),
                };
                let _ = serde_json::to_string(&resp).unwrap();
            }
        }
    }
    let e3 = t3.elapsed();
    println!(
        "  warm clone+json x10000: {:?} ({:.1} us/lookup)",
        e3,
        e3.as_micros() as f64 / 10000.0
    );

    // Also test words NOT in dictionary (miss path)
    let misses = ["xxxxx", "yyy", "zzzzz", "qqq", "nonexistent"];
    let t4 = Instant::now();
    for _ in 0..10000 {
        for w in &misses {
            let _ = r.lookup(w);
        }
    }
    let e4 = t4.elapsed();
    println!(
        "  miss x50000: {:?} ({:.1} us/lookup)",
        e4,
        e4.as_micros() as f64 / 50000.0
    );
}

fn main() {
    let path = std::env::args().nth(1).expect("path");
    bench(&path, ReadMode::Eager, "EAGER");
    bench(&path, ReadMode::Lazy, "LAZY");
}
