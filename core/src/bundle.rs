//! HD1 bundle files: a zlib-chunked payload holding an index and resource entries.

use std::path::Path;

use crate::{ObsidianError, Result};

pub const VERSION: u32 = 0xF000_0004;
const CHUNK: usize = 0x10000;
const PROPS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub lang: u32,
    pub data: Vec<u8>,
    pub stream_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub type_hash: u64,
    pub name_hash: u64,
    pub flags: u32,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    pub props: [u8; PROPS],
    pub entries: Vec<Entry>,
}

/// Reads little-endian values from a byte slice, failing with `TruncatedBundle`.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(ObsidianError::TruncatedBundle)?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

/// Inflates chunks from `file` (header included) until `want` payload bytes are out, or all of them.
fn inflate(file: &[u8], want: Option<usize>) -> Result<Vec<u8>> {
    let mut c = Cursor { data: file, pos: 0 };
    let version = c.u32()?;
    if version != VERSION {
        return Err(ObsidianError::BadBundleVersion(version));
    }
    let total = c.u32()? as usize;
    c.u32()?;
    let goal = want.map_or(total, |w| w.min(total));
    // The header can say anything, so it doesn't choose the allocation.
    let mut out = Vec::with_capacity(goal.min(file.len().saturating_mul(8)));
    while out.len() < goal {
        let size = c.u32()? as usize;
        let chunk = c.take(size)?;
        if size == CHUNK {
            out.extend_from_slice(chunk);
        } else {
            use std::io::Read;
            flate2::read::ZlibDecoder::new(chunk).take(CHUNK as u64).read_to_end(&mut out).map_err(|_| ObsidianError::TruncatedBundle)?;
        }
    }
    out.truncate(total);
    Ok(out)
}

pub fn decompress(file: &[u8]) -> Result<Vec<u8>> {
    inflate(file, None)
}

pub fn compress(payload: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::with_capacity(payload.len() / 2 + 64);
    out.extend(VERSION.to_le_bytes());
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(0u32.to_le_bytes());
    for piece in payload.chunks(CHUNK) {
        let mut block = piece.to_vec();
        block.resize(CHUNK, 0);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&block).unwrap();
        let z = z.finish().unwrap();
        let stored = z.len() >= CHUNK;
        let body = if stored { &block } else { &z };
        out.extend((body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
    }
    out
}

pub fn parse_payload(payload: &[u8]) -> Result<Bundle> {
    let mut c = Cursor { data: payload, pos: 0 };
    let count = c.u32()? as usize;
    let props: [u8; PROPS] = c.take(PROPS)?.try_into().unwrap();
    // The index repeats what each entry header says, so skip it.
    c.take(count.checked_mul(16).ok_or(ObsidianError::TruncatedBundle)?)?;
    // Counts come from the file, so they don't choose an allocation either.
    let mut entries = Vec::with_capacity(count.min(payload.len() / 24));
    for _ in 0..count {
        let type_hash = c.u64()?;
        let name_hash = c.u64()?;
        let n = c.u32()? as usize;
        let flags = c.u32()?;
        let mut heads = Vec::with_capacity(n.min(payload.len() / 12));
        for _ in 0..n {
            heads.push((c.u32()?, c.u32()? as usize, c.u32()?));
        }
        let variants = heads
            .into_iter()
            .map(|(lang, size, stream_size)| Ok(Variant { lang, data: c.take(size)?.to_vec(), stream_size }))
            .collect::<Result<Vec<_>>>()?;
        entries.push(Entry { type_hash, name_hash, flags, variants });
    }
    if payload[c.pos..].iter().any(|&b| b != 0) {
        return Err(ObsidianError::TrailingData);
    }
    Ok(Bundle { props, entries })
}

impl Bundle {
    /// Sorts entries by (type, name), the order most vanilla bundles use.
    pub fn sort_entries(&mut self) {
        self.entries.sort_by_key(|e| (e.type_hash, e.name_hash));
    }
}

/// Writes entries in the order given: three vanilla bundles keep an unsorted index.
pub fn serialize_payload(b: &Bundle) -> Vec<u8> {
    let entries: Vec<&Entry> = b.entries.iter().collect();
    let mut out = Vec::new();
    out.extend((entries.len() as u32).to_le_bytes());
    out.extend_from_slice(&b.props);
    for e in &entries {
        out.extend(e.type_hash.to_le_bytes());
        out.extend(e.name_hash.to_le_bytes());
    }
    for e in &entries {
        out.extend(e.type_hash.to_le_bytes());
        out.extend(e.name_hash.to_le_bytes());
        out.extend((e.variants.len() as u32).to_le_bytes());
        out.extend(e.flags.to_le_bytes());
        for v in &e.variants {
            out.extend(v.lang.to_le_bytes());
            out.extend((v.data.len() as u32).to_le_bytes());
            out.extend(v.stream_size.to_le_bytes());
        }
        for v in &e.variants {
            out.extend_from_slice(&v.data);
        }
    }
    out
}

pub fn read_file(path: &Path) -> Result<Bundle> {
    parse_payload(&decompress(&std::fs::read(path)?)?)
}

/// The properties of a bundle, reading only the start of the file: a game bundle can pass a gigabyte.
pub fn read_props(path: &Path) -> Result<[u8; PROPS]> {
    use std::io::Read;
    let mut head = Vec::new();
    std::fs::File::open(path)?.take(16 + CHUNK as u64).read_to_end(&mut head)?;
    let payload = inflate(&head, Some(4 + PROPS))?;
    payload.get(4..4 + PROPS).and_then(|p| p.try_into().ok()).ok_or(ObsidianError::TruncatedBundle)
}

/// The bytes of `b`'s payload, in the order `serialize_payload` writes them, without joining them: a patch
/// of sound banks can pass a gigabyte.
fn payload_parts(b: &Bundle) -> Vec<std::borrow::Cow<'_, [u8]>> {
    use std::borrow::Cow::{Borrowed, Owned};
    let mut head = Vec::with_capacity(4 + PROPS + b.entries.len() * 16);
    head.extend((b.entries.len() as u32).to_le_bytes());
    head.extend_from_slice(&b.props);
    for e in &b.entries {
        head.extend(e.type_hash.to_le_bytes());
        head.extend(e.name_hash.to_le_bytes());
    }
    let mut parts = vec![Owned(head)];
    for e in &b.entries {
        let mut entry = Vec::with_capacity(24 + e.variants.len() * 12);
        entry.extend(e.type_hash.to_le_bytes());
        entry.extend(e.name_hash.to_le_bytes());
        entry.extend((e.variants.len() as u32).to_le_bytes());
        entry.extend(e.flags.to_le_bytes());
        for v in &e.variants {
            entry.extend(v.lang.to_le_bytes());
            entry.extend((v.data.len() as u32).to_le_bytes());
            entry.extend(v.stream_size.to_le_bytes());
        }
        parts.push(Owned(entry));
        parts.extend(e.variants.iter().map(|v| Borrowed(v.data.as_slice())));
    }
    parts
}

/// One 64 KB block of a bundle file: compressed, or None when that's no smaller and it's stored.
fn pack_block(block: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(block)?;
    let z = z.finish()?;
    Ok((z.len() < CHUNK).then_some(z))
}

/// How many blocks wait to be compressed together, per core. A patch can pass a gigabyte, and compressing
/// it one block at a time takes over a minute.
const BLOCKS_PER_CORE: usize = 64;

/// Compresses the waiting blocks on every core and writes them in their order.
fn flush_blocks(out: &mut impl std::io::Write, blocks: &mut Vec<Vec<u8>>) -> std::io::Result<()> {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let per = blocks.len().div_ceil(cores).max(1);
    let groups: Vec<std::io::Result<Vec<Option<Vec<u8>>>>> = std::thread::scope(|s| {
        let running: Vec<_> = blocks.chunks(per).map(|group| s.spawn(move || group.iter().map(|b| pack_block(b)).collect())).collect();
        running.into_iter().map(|t| t.join().expect("compressing a block doesn't panic")).collect()
    });
    let mut plain = blocks.iter();
    for group in groups {
        for packed in group? {
            let block = plain.next().expect("one result per block");
            let body = packed.as_deref().unwrap_or(block);
            out.write_all(&(body.len() as u32).to_le_bytes())?;
            out.write_all(body)?;
        }
    }
    blocks.clear();
    Ok(())
}

pub fn write_file(path: &Path, b: &Bundle) -> Result<()> {
    write_file_with(path, b, &mut |_| {})
}

/// `write_file`, saying now and then how far it is, from 0 to 1.
pub fn write_file_with(path: &Path, b: &Bundle, progress: &mut dyn FnMut(f32)) -> Result<()> {
    use std::io::Write;
    let parts = payload_parts(b);
    let total: u64 = parts.iter().map(|p| p.len() as u64).sum();
    let too_big = || ObsidianError::PatchTooLarge(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
    let total = u32::try_from(total).map_err(|_| too_big())?;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    out.write_all(&VERSION.to_le_bytes())?;
    out.write_all(&total.to_le_bytes())?;
    out.write_all(&0u32.to_le_bytes())?;
    let batch = BLOCKS_PER_CORE * std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(batch);
    let mut block = Vec::with_capacity(CHUNK);
    let mut done = 0u64;
    for part in &parts {
        let mut rest: &[u8] = part;
        while !rest.is_empty() {
            let (now, later) = rest.split_at(rest.len().min(CHUNK - block.len()));
            block.extend_from_slice(now);
            rest = later;
            if block.len() == CHUNK {
                blocks.push(std::mem::replace(&mut block, Vec::with_capacity(CHUNK)));
                if blocks.len() == batch {
                    flush_blocks(&mut out, &mut blocks)?;
                    done += (batch * CHUNK) as u64;
                    progress(done as f32 / total.max(1) as f32);
                }
            }
        }
    }
    if !block.is_empty() {
        block.resize(CHUNK, 0);
        blocks.push(block);
    }
    flush_blocks(&mut out, &mut blocks)?;
    out.flush()?;
    progress(1.0);
    Ok(())
}

/// The (type, name) index, inflating only the chunks that hold it.
pub fn read_index(path: &Path) -> Result<Vec<(u64, u64)>> {
    use std::io::Read;
    // Only the start of the file: a game bundle can pass a gigabyte.
    let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut head = [0u8; 12];
    file.read_exact(&mut head)?;
    let version = u32::from_le_bytes(head[..4].try_into().unwrap());
    if version != VERSION {
        return Err(ObsidianError::BadBundleVersion(version));
    }
    let mut payload = Vec::new();
    let mut want = 4 + PROPS;
    let mut counted = false;
    while payload.len() < want {
        let mut size = [0u8; 4];
        file.read_exact(&mut size)?;
        let mut body = vec![0u8; u32::from_le_bytes(size) as usize];
        file.read_exact(&mut body)?;
        payload.extend(unpack_block(&body)?);
        if !counted {
            let count = u32::from_le_bytes(payload[..4].try_into().map_err(|_| ObsidianError::TruncatedBundle)?) as usize;
            want += count.checked_mul(16).ok_or(ObsidianError::TruncatedBundle)?;
            counted = true;
        }
    }
    let mut c = Cursor { data: &payload, pos: 4 + PROPS };
    let count = (want - 4 - PROPS) / 16;
    (0..count).map(|_| Ok((c.u64()?, c.u64()?))).collect()
}

fn unpack_block(body: &[u8]) -> Result<Vec<u8>> {
    if body.len() == CHUNK {
        return Ok(body.to_vec());
    }
    use std::io::Read;
    let mut out = Vec::with_capacity(CHUNK);
    flate2::read::ZlibDecoder::new(body).take(CHUNK as u64).read_to_end(&mut out).map_err(|_| ObsidianError::TruncatedBundle)?;
    Ok(out)
}

/// Where one language of a resource is: in the bundle's payload, and in its `.stream`.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantHead {
    pub lang: u32,
    pub at: u64,
    pub size: u32,
    pub stream_at: u64,
    pub stream_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryHead {
    pub type_hash: u64,
    pub name_hash: u64,
    pub flags: u32,
    pub variants: Vec<VariantHead>,
}

/// A bundle file read in parts, for what's too big to hold whole.
pub struct Reader {
    file: std::io::BufReader<std::fs::File>,
    /// Where each 64 KB block's bytes are in the file, and how many.
    blocks: Vec<(u64, u32)>,
    total: u64,
    last: Option<(usize, Vec<u8>)>,
}

impl Reader {
    pub fn open(path: &Path) -> Result<Reader> {
        use std::io::Read;
        let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut head = [0u8; 12];
        file.read_exact(&mut head)?;
        let version = u32::from_le_bytes(head[..4].try_into().unwrap());
        if version != VERSION {
            return Err(ObsidianError::BadBundleVersion(version));
        }
        let total = u32::from_le_bytes(head[4..8].try_into().unwrap()) as u64;
        let mut blocks = Vec::new();
        let mut at = 12u64;
        for _ in 0..total.div_ceil(CHUNK as u64) {
            let mut size = [0u8; 4];
            file.read_exact(&mut size)?;
            let size = u32::from_le_bytes(size);
            blocks.push((at + 4, size));
            file.seek_relative(size as i64)?;
            at += 4 + size as u64;
        }
        Ok(Reader { file, blocks, total, last: None })
    }

    /// `n` bytes of the payload from `at`.
    pub fn read(&mut self, at: u64, n: usize) -> Result<Vec<u8>> {
        use std::io::{Read, Seek, SeekFrom};
        if at + n as u64 > self.total {
            return Err(ObsidianError::TruncatedBundle);
        }
        let mut out = Vec::with_capacity(n);
        let mut at = at as usize;
        while out.len() < n {
            let i = at / CHUNK;
            if self.last.as_ref().is_none_or(|(have, _)| *have != i) {
                let (pos, size) = *self.blocks.get(i).ok_or(ObsidianError::TruncatedBundle)?;
                self.file.seek(SeekFrom::Start(pos))?;
                let mut body = vec![0u8; size as usize];
                self.file.read_exact(&mut body)?;
                self.last = Some((i, unpack_block(&body)?));
            }
            let block = &self.last.as_ref().unwrap().1;
            let from = at % CHUNK;
            let part = block.get(from..(from + n - out.len()).min(block.len())).filter(|p| !p.is_empty()).ok_or(ObsidianError::TruncatedBundle)?;
            out.extend_from_slice(part);
            at += part.len();
        }
        Ok(out)
    }

    /// Every resource of the bundle, with its place. A `.stream` holds each language's part in this
    /// order, so finding one part takes walking them all.
    pub fn heads(&mut self) -> Result<Vec<EntryHead>> {
        let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        let count = word(&self.read(0, 4)?, 0) as u64;
        let mut at = 4 + PROPS as u64 + count * 16;
        let mut stream_at = 0u64;
        let mut out = Vec::new();
        for _ in 0..count {
            let h = self.read(at, 24)?;
            let n = word(&h, 16) as usize;
            at += 24;
            let list = self.read(at, n.checked_mul(12).ok_or(ObsidianError::TruncatedBundle)?)?;
            at += n as u64 * 12;
            let mut variants = Vec::with_capacity(n);
            for v in list.chunks(12) {
                let (size, stream_size) = (word(v, 4), word(v, 8));
                variants.push(VariantHead { lang: word(v, 0), at, size, stream_at, stream_size });
                at += size as u64;
                stream_at += stream_size as u64;
            }
            out.push(EntryHead {
                type_hash: u64::from_le_bytes(h[..8].try_into().unwrap()),
                name_hash: u64::from_le_bytes(h[8..16].try_into().unwrap()),
                flags: word(&h, 20),
                variants,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_written_in_parts_is_the_same_as_one_written_whole() {
        let t = crate::testutil::spaced_tempdir();
        let path = t.path().join("b");
        // Past one block, with a piece that crosses from one into the next.
        let big: Vec<u8> = (0..200_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let b = Bundle { props: [3; PROPS], entries: vec![entry(2, 1, 0, b"small"), entry(9, 9, 4, &big), entry(1, 5, 0, b"")] };
        write_file(&path, &b).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), compress(&serialize_payload(&b)));
    }

    #[test]
    fn the_reader_finds_each_language_and_its_place_in_the_stream() {
        let t = crate::testutil::spaced_tempdir();
        let path = t.path().join("b");
        let big: Vec<u8> = (0..150_000u32).map(|i| (i % 253) as u8).collect();
        let v = |lang, data: &[u8], stream_size| Variant { lang, data: data.to_vec(), stream_size };
        let b = Bundle { props: [3; PROPS], entries: vec![
            Entry { type_hash: 1, name_hash: 1, flags: 0, variants: vec![v(0, b"first", 5)] },
            Entry { type_hash: 2, name_hash: 2, flags: 7, variants: vec![v(0, &big, 10), v(256, b"fr", 3)] },
        ] };
        write_file(&path, &b).unwrap();
        let mut r = Reader::open(&path).unwrap();
        let heads = r.heads().unwrap();
        assert_eq!(heads.len(), 2);
        assert_eq!((heads[1].flags, heads[1].variants[0].stream_at, heads[1].variants[1].stream_at), (7, 5, 15));
        let en = heads[1].variants[0].clone();
        assert_eq!(r.read(en.at, en.size as usize).unwrap(), big, "across blocks");
        let fr = heads[1].variants[1].clone();
        assert_eq!(r.read(fr.at, fr.size as usize).unwrap(), b"fr");
        assert!(matches!(r.read(1 << 40, 1), Err(ObsidianError::TruncatedBundle)));
    }

    fn entry(type_hash: u64, name_hash: u64, flags: u32, data: &[u8]) -> Entry {
        Entry { type_hash, name_hash, flags, variants: vec![Variant { lang: 0, data: data.to_vec(), stream_size: 0 }] }
    }

    fn sample() -> Bundle {
        Bundle { props: [7u8; PROPS], entries: vec![entry(2, 9, 0, b"bb"), entry(1, 5, 531101911, b"aaaa")] }
    }

    #[test]
    fn props_come_from_the_start_of_the_file_alone() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("b");
        write_file(&path, &sample()).unwrap();
        assert_eq!(read_props(&path).unwrap(), [7u8; PROPS]);
    }

    #[test]
    fn a_count_in_the_file_cant_choose_a_huge_allocation() {
        let mut p = 0xFFFF_FFFFu32.to_le_bytes().to_vec();
        p.extend([0u8; PROPS]);
        assert!(parse_payload(&p).is_err());
    }

    #[test]
    fn payload_round_trips_in_given_order() {
        let p = serialize_payload(&sample());
        let back = parse_payload(&p).unwrap();
        assert_eq!(back, sample());
        assert_eq!(back.entries[1].flags, 531101911);
        assert_eq!(serialize_payload(&back), p);
    }

    #[test]
    fn sort_entries_orders_by_type_then_name() {
        let mut b = sample();
        b.sort_entries();
        assert_eq!((b.entries[0].type_hash, b.entries[1].type_hash), (1, 2));
    }

    #[test]
    fn many_blocks_keep_their_order_and_progress_reaches_the_end() {
        // Past several batches of blocks, part of it not compressible, so some blocks are stored as they are.
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let size = (BLOCKS_PER_CORE * cores * 3 + 7) * CHUNK + 123;
        let mut x = 0x2545f4914f6cdd1du64;
        let data: Vec<u8> = (0..size)
            .map(|i| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                if i % (CHUNK * 2) < CHUNK { x as u8 } else { (i / CHUNK) as u8 }
            })
            .collect();
        let b = Bundle { props: [3; 256], entries: vec![Entry { type_hash: 1, name_hash: 2, flags: 0, variants: vec![Variant { lang: 0, data, stream_size: 0 }] }] };
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("big");
        let mut seen = Vec::new();
        write_file_with(&path, &b, &mut |part| seen.push(part)).unwrap();
        assert_eq!(read_file(&path).unwrap(), b);
        assert!(seen.len() >= 3, "{seen:?}");
        assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
        assert_eq!(*seen.last().unwrap(), 1.0);
    }

    #[test]
    fn file_round_trips_through_compression() {
        let mut b = sample();
        // An xorshift sequence does not compress, so this exercises stored chunks too.
        let mut x: u32 = 0x1234_5678;
        let noise: Vec<u8> = (0..200_000).map(|_| { x ^= x << 13; x ^= x >> 17; x ^= x << 5; x as u8 }).collect();
        b.entries.push(entry(3, 3, 0, &noise));
        let back = parse_payload(&decompress(&compress(&serialize_payload(&b))).unwrap()).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn zero_padding_after_entries_is_accepted() {
        let mut p = serialize_payload(&sample());
        p.extend([0u8; 64]);
        assert_eq!(parse_payload(&p).unwrap().entries.len(), 2);
    }

    #[test]
    fn nonzero_trailing_data_is_rejected() {
        let mut p = serialize_payload(&sample());
        p.push(1);
        assert!(matches!(parse_payload(&p), Err(ObsidianError::TrailingData)));
    }

    #[test]
    fn wrong_version_is_rejected() {
        let mut f = compress(&serialize_payload(&sample()));
        f[..4].copy_from_slice(&0xF000_0005u32.to_le_bytes());
        assert!(matches!(decompress(&f), Err(ObsidianError::BadBundleVersion(0xF000_0005))));
    }

    #[test]
    fn index_reader_matches_full_parse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("00000000000000aa");
        write_file(&path, &sample()).unwrap();
        let expected: Vec<_> = read_file(&path).unwrap().entries.iter().map(|e| (e.type_hash, e.name_hash)).collect();
        assert_eq!(read_index(&path).unwrap(), expected);
    }
}
