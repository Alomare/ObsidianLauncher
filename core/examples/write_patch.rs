//! Times writing one patch the way a deploy does: `cargo run --release -p obsidian-core --example
//! write_patch -- <patch> <out file>`.

use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let (from, to) = (args.next().expect("a patch"), args.next().expect("an out file"));
    let b = obsidian_core::bundle::read_file(std::path::Path::new(&from)).unwrap();
    for _ in 0..3 {
        let start = Instant::now();
        obsidian_core::bundle::write_file_with(std::path::Path::new(&to), &b, &mut |_| {}).unwrap();
        println!("write: {:.0} ms", start.elapsed().as_secs_f64() * 1000.0);
    }
    std::fs::remove_file(&to).unwrap();
}
