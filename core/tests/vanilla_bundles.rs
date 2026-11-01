//! Needs the real game: set OBSIDIAN_GAME_DIR to the Helldivers folder and run with --ignored.

use obsidian_core::bundle;

#[test]
#[ignore]
fn every_vanilla_bundle_round_trips() {
    let game = std::env::var("OBSIDIAN_GAME_DIR").expect("OBSIDIAN_GAME_DIR");
    let contents = std::path::Path::new(&game).join("contents");
    let (mut bundles, mut entries) = (0, 0);
    for f in std::fs::read_dir(&contents).unwrap() {
        let path = f.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.len() != 16 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let payload = bundle::decompress(&std::fs::read(&path).unwrap()).unwrap();
        let parsed = bundle::parse_payload(&payload).unwrap();
        let again = bundle::serialize_payload(&parsed);
        assert_eq!(&payload[..again.len()], &again[..], "{name}");
        assert!(payload[again.len()..].iter().all(|&b| b == 0), "{name}");
        assert_eq!(bundle::read_index(&path).unwrap().len(), parsed.entries.len(), "{name}");
        bundles += 1;
        entries += parsed.entries.len();
    }
    println!("{bundles} bundles, {entries} entries");
    assert_eq!((bundles, entries), (84, 37459));
}

/// Every language of every bank of the game: rebuilt with nothing new it's the same bytes, the reader agrees
/// with the whole bundle, and every clip passes the check a mod's clips have to.
#[test]
#[ignore]
fn every_vanilla_bank_rebuilds_byte_for_byte() {
    use obsidian_core::bank;
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::{Read, Seek, SeekFrom};
    let game = std::env::var("OBSIDIAN_GAME_DIR").expect("OBSIDIAN_GAME_DIR");
    let contents = std::path::Path::new(&game).join("contents");
    let timpani = obsidian_core::hash::hash_name("timpani_bank");
    let (mut seen, mut clips) = (BTreeSet::new(), 0);
    for f in std::fs::read_dir(&contents).unwrap() {
        let path = f.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.len() != 16 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let mut r = bundle::Reader::open(&path).unwrap();
        let heads = r.heads().unwrap();
        assert_eq!(heads.iter().map(|e| (e.type_hash, e.name_hash)).collect::<Vec<_>>(), bundle::read_index(&path).unwrap(), "{name}");
        for e in heads.iter().filter(|e| e.type_hash == timpani) {
            for v in &e.variants {
                if !seen.insert((e.name_hash, v.lang)) {
                    continue;
                }
                let raw = r.read(v.at, v.size as usize).unwrap();
                let mut stream = vec![0u8; v.stream_size as usize];
                if v.stream_size > 0 {
                    let mut s = std::fs::File::open(path.with_file_name(format!("{name}.stream"))).unwrap();
                    s.seek(SeekFrom::Start(v.stream_at)).unwrap();
                    s.read_exact(&mut stream).unwrap();
                }
                let (again, again_stream) = bank::rebuild(&raw, &stream, &BTreeMap::new()).unwrap();
                assert!(again == raw && again_stream == stream, "{:016x} {:x}", e.name_hash, v.lang);
                for row in bank::rows_of(&raw).unwrap() {
                    bank::check_clip(&raw[row.off..row.off + row.size]).unwrap_or_else(|why| panic!("{:016x} {:x} {:016x}: {why}", e.name_hash, v.lang, row.clip));
                    clips += 1;
                }
            }
        }
    }
    println!("{} banks and languages, {clips} clips", seen.len());
    assert_eq!(seen.len(), 56);
}

/// How long the biggest bank takes to rebuild and write, and that what's written reads back.
#[test]
#[ignore]
fn the_biggest_bank_rebuilds_into_a_patch_in_reasonable_time() {
    use obsidian_core::bank;
    use std::collections::BTreeMap;
    use std::io::{Read, Seek, SeekFrom};
    let game = std::env::var("OBSIDIAN_GAME_DIR").expect("OBSIDIAN_GAME_DIR");
    let contents = std::path::Path::new(&game).join("contents");
    let timpani = obsidian_core::hash::hash_name("timpani_bank");
    let name = 0xe0f28740d6c19769u64;
    let started = std::time::Instant::now();
    let holder = std::fs::read_dir(&contents).unwrap().map(|f| f.unwrap().path()).find(|p| {
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        n.len() == 16 && !n.contains('.') && bundle::read_index(p).unwrap().contains(&(timpani, name))
    }).expect("the bank");
    println!("found its bundle in {:?}", started.elapsed());
    let mut r = bundle::Reader::open(&holder).unwrap();
    let heads = r.heads().unwrap();
    let e = heads.iter().find(|e| e.name_hash == name).unwrap();
    let mut s = std::fs::File::open(format!("{}.stream", holder.display())).unwrap();
    let mut variants = Vec::new();
    let mut stream_total = 0u64;
    for v in &e.variants {
        let raw = r.read(v.at, v.size as usize).unwrap();
        let mut stream = vec![0u8; v.stream_size as usize];
        s.seek(SeekFrom::Start(v.stream_at)).unwrap();
        s.read_exact(&mut stream).unwrap();
        let row = bank::rows_of(&raw).unwrap()[0];
        let mut clip = raw[row.off..row.off + row.size].to_vec();
        clip.extend_from_slice(b"");
        let new = BTreeMap::from([(row.clip, (clip.as_slice(), &b"swapped"[..]))]);
        let (out, out_stream) = bank::rebuild(&raw, &stream, &new).unwrap();
        stream_total += out_stream.len() as u64;
        variants.push(bundle::Variant { lang: v.lang, stream_size: out_stream.len() as u32, data: out });
    }
    println!("read and rebuilt in {:?}", started.elapsed());
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("patch");
    let b = bundle::Bundle { props: bundle::read_props(&holder).unwrap(), entries: vec![bundle::Entry { type_hash: timpani, name_hash: name, flags: e.flags, variants }] };
    let write = std::time::Instant::now();
    bundle::write_file(&path, &b).unwrap();
    println!("wrote {} MB (and {} MB of stream to follow) in {:?}, {} MB on disk", b.entries[0].variants.iter().map(|v| v.data.len()).sum::<usize>() / 1_000_000, stream_total / 1_000_000, write.elapsed(), std::fs::metadata(&path).unwrap().len() / 1_000_000);
    let mut back = bundle::Reader::open(&path).unwrap();
    let again = back.heads().unwrap();
    assert_eq!(again[0].variants.len(), b.entries[0].variants.len());
    let v = &again[0].variants[0];
    assert_eq!(back.read(v.at, v.size as usize).unwrap(), b.entries[0].variants[0].data);
}
