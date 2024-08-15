//! A simple crate for counting events in Rust libraries.
//!
//! This library is designed to be used in performance-sensitive code to gather statistics about the
//! frequency of events. It can handle tens of millions of events per second on a single core with
//! manageable performance overhead. Capturing events doesn't require passing any global context,
//! making it well suited to instrument and optimize low level libraries within a larger program.
//! Events are accumulated into thread-local counters and can be printed out at the end of the
//! program.
//!
//! # Example
//!
//! ```rust,no_run
//! // Capture an event
//! innumerable::event!("event_name", 12);
//!
//! // At program completion, print out the counts
//! innumerable::print_counts();
//! ```

#![forbid(unsafe_code)]
#![cfg_attr(test, feature(test))]
#[cfg(test)]
extern crate test;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

type HashMap<K, V> = hashbrown::HashMap<K, V>;
type Map = Arc<Mutex<HashMap<(&'static str, i64), u64>>>;

lazy_static::lazy_static! {
    static ref MAPS: Mutex<Vec<Map>> = Default::default();
}

fn create_local_map() -> Map {
    let map = Arc::new(Mutex::new(HashMap::default()));
    MAPS.lock().unwrap().push(map.clone());
    map
}

thread_local! {
    #[doc(hidden)]
    pub static THREAD_COUNTS: Map = create_local_map();
}

/// Count an event.
///
/// The first argument is the name of the event, and the second argument is an optional index.
#[macro_export]
macro_rules! event {
    ($name:expr, $index:expr) => {
        $crate::THREAD_COUNTS.with(|counts| {
            *counts
                .lock()
                .unwrap()
                .entry(($name, $index as i64))
                .or_insert(0) += 1;
        })
    };
    ($name:expr) => {
        $crate::event!($name, 0);
    };
}

/// Print out the counts of all events.
pub fn print_counts() {
    let maps = MAPS.lock().unwrap();
    let mut events = BTreeMap::new();
    for map in &*maps {
        let map = map.lock().unwrap();
        for (&(name, index), &count) in map.iter() {
            events
                .entry(name)
                .or_insert_with(BTreeMap::new)
                .insert(index, count);
        }
    }

    for (name, counts) in events.iter() {
        let sum = counts.values().copied().sum::<u64>() as f64;
        println!("{name}: {sum}");
        if counts.len() > 1 || !counts.contains_key(&0) {
            for (index, &count) in counts.iter() {
                println!("{name}[{index}]: {:.1}%", count as f64 / sum * 100.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};

    #[bench]
    fn bench_event(b: &mut Bencher) {
        b.iter(|| event!(black_box("testevent"), black_box(5)));
    }
}
