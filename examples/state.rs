//! Versioned values and memoized derived state without a window.
use dga_bevy_wings_framework::prelude::*;

fn main() {
    let mut count = WingsState::new(1);
    let mut doubled = WingsMemo::default();
    for next in [1, 2, 2, 3] {
        let changed = count.set(next);
        let value = doubled.get_or_compute(count.version(), || {
            println!("Computing at version {}", count.version());
            count.get() * 2
        });
        println!("count={next}, changed={changed}, doubled={value}");
    }
}
