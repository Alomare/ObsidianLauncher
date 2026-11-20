//! Compresses the payload of a patch at each zlib level, on every core as a deploy does:
//! `cargo run --release -p obsidian-core --example zlib_levels -- <patch>`.

use std::io::Write;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("a patch");
    let file = std::fs::read(&path).unwrap();
    let payload = obsidian_core::bundle::decompress(&file).unwrap();
    let blocks: Vec<&[u8]> = payload.chunks(0x10000).collect();
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("payload {:.1} MB, file {:.1} MB, {cores} cores", payload.len() as f64 / 1e6, file.len() as f64 / 1e6);
    for level in [0u32, 1, 2, 3, 4, 6] {
        let start = Instant::now();
        let per = blocks.len().div_ceil(cores);
        let total: usize = std::thread::scope(|s| {
            let running: Vec<_> = blocks
                .chunks(per)
                .map(|group| {
                    s.spawn(move || {
                        group
                            .iter()
                            .map(|b| {
                                let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
                                z.write_all(b).unwrap();
                                z.finish().unwrap().len().min(0x10000)
                            })
                            .sum::<usize>()
                    })
                })
                .collect();
            running.into_iter().map(|t| t.join().unwrap()).sum()
        });
        println!("level {level}: {:>7.0} ms, {:>6.1} MB", start.elapsed().as_secs_f64() * 1000.0, total as f64 / 1e6);
    }
}
