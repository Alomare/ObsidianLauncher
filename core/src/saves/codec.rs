//! The game's save files, local and Steam Cloud: the container (crc32 and zlib chunks) and the tagged
//! table inside. A decoded save is a tree that keeps every key, in the order the game wrote them, so
//! encoding gives back the same plain bytes.

use std::io::{Read, Write};
use std::path::Path;

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::{ObsidianError, Result};

/// Plain bytes in every compressed chunk.
pub const CHUNK: usize = 65536;
const CLOUD_MAGIC: [u8; 8] = [0xd4, 0xfd, 0x1c, 0x6b, 0x5f, 0x7b, 0x23, 0xcc];
/// What a Cloud file holds before the plain data a local file has.
const CLOUD_PREFIX: [u8; 8] = [0x0e, 0xbf, 0x53, 0x40, 0x8c, 0x67, 0xde, 0x03];
/// The most the plain data may be: far above a real save, and it bounds a bad import.
const MAX_PLAIN: usize = 1 << 26;
const MAX_DEPTH: u32 = 128;

const TAG_TABLE: u32 = 0;
const TAG_CLOSE: u32 = 1;
const TAG_TEXT: u32 = 3;
const TAG_BOOL: u32 = 4;
const TAG_NUMBER: u32 = 9;

fn bad(why: impl Into<String>) -> ObsidianError {
    ObsidianError::BadSave(why.into())
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = flate2::Crc::new();
    c.update(bytes);
    c.sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Local,
    Cloud,
}

/// A value in a save. Text is bytes, as the game wrote them; a bool keeps the u32 on disk.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Table(Table),
    Text(Vec<u8>),
    Bool(u32),
    Number(f64),
}

impl Value {
    pub fn text(s: &str) -> Value {
        Value::Text(s.as_bytes().to_vec())
    }

    pub fn as_str(&self) -> Option<String> {
        match self {
            Value::Text(b) => Some(String::from_utf8_lossy(b).into_owned()),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b != 0),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }
}

/// A table: (key, value) pairs in file order. A key is text, a number or a bool.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table(pub Vec<(Value, Value)>);

fn key_is(k: &Value, name: &str) -> bool {
    matches!(k, Value::Text(b) if b.as_slice() == name.as_bytes())
}

impl Table {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| key_is(k, key)).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.iter_mut().find(|(k, _)| key_is(k, key)).map(|(_, v)| v)
    }

    /// Replaces the value of `key`, or adds the key at the end.
    pub fn set(&mut self, key: &str, value: Value) {
        match self.get_mut(key) {
            Some(slot) => *slot = value,
            None => self.0.push((Value::text(key), value)),
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.0.retain(|(k, _)| !key_is(k, key));
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(Value::as_number)
    }

    pub fn table(&self, key: &str) -> Option<&Table> {
        self.get(key).and_then(Value::as_table)
    }

    /// The table under `key`, added empty when absent or not a table.
    pub fn table_mut(&mut self, key: &str) -> &mut Table {
        if !self.get(key).is_some_and(|v| matches!(v, Value::Table(_))) {
            self.set(key, Value::Table(Table::default()));
        }
        self.get_mut(key).and_then(Value::as_table_mut).expect("set above")
    }

    /// The text values of this table in key order, numbers first in file order: how the game keeps
    /// a list of names.
    pub fn strings(&self) -> Vec<String> {
        self.0.iter().filter_map(|(_, v)| v.as_str()).collect()
    }

    /// Makes this table a list of `names`, keyed 1, 2, 3 and so on as the game writes one.
    pub fn set_strings(&mut self, names: &[String]) {
        self.0 = names.iter().enumerate().map(|(i, n)| (Value::Number((i + 1) as f64), Value::text(n))).collect();
    }

    /// The number stored at list position `i` (1-based).
    pub fn at(&self, i: usize) -> Option<&Value> {
        self.0.iter().find(|(k, _)| matches!(k, Value::Number(n) if *n == i as f64)).map(|(_, v)| v)
    }

    pub fn set_at(&mut self, i: usize, value: Value) {
        match self.0.iter_mut().find(|(k, _)| matches!(k, Value::Number(n) if *n == i as f64)) {
            Some((_, v)) => *v = value,
            None => self.0.push((Value::Number(i as f64), value)),
        }
    }
}

/// One decoded save file. `header` is the four words after the length at the start of the plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveFile {
    pub header: [u32; 4],
    pub root: Table,
}

impl SaveFile {
    /// An empty table with the header every save has.
    pub fn empty() -> SaveFile {
        SaveFile { header: [0, 1, 0, 0], root: Table::default() }
    }
}

pub fn read_file(path: &Path) -> Result<SaveFile> {
    decode_file(&std::fs::read(path)?).map_err(|e| match e {
        ObsidianError::BadSave(why) => ObsidianError::BadSave(format!("{}: {why}", path.display())),
        other => other,
    })
}

fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    let w = b.get(p..p + 4).ok_or_else(|| bad("the file ends early"))?;
    Ok(u32::from_le_bytes(w.try_into().unwrap()))
}

/// Reads a whole file of either kind, checking the crc.
pub fn decode_file(bytes: &[u8]) -> Result<SaveFile> {
    let (format, body_at) = if bytes.starts_with(&CLOUD_MAGIC) { (Format::Cloud, 16) } else { (Format::Local, 4) };
    let crc = u32_at(bytes, body_at - 4)?;
    if bytes.len() < body_at + 16 {
        return Err(bad("the file ends early"));
    }
    if crc32(&bytes[body_at..]) != crc {
        return Err(bad("the checksum doesn't match"));
    }
    let size = u32_at(bytes, body_at + 4)? as usize;
    if size > MAX_PLAIN {
        return Err(bad("the save is too large"));
    }
    let mut plain = Vec::with_capacity(size);
    let mut p = body_at + 16;
    while p < bytes.len() {
        let zlen = u32_at(bytes, p)? as usize;
        p += 4;
        let chunk = bytes.get(p..p + zlen).ok_or_else(|| bad("a chunk runs past the end"))?;
        p += zlen;
        let mut out = Vec::with_capacity(CHUNK);
        ZlibDecoder::new(chunk).take(CHUNK as u64 + 1).read_to_end(&mut out).map_err(|e| bad(format!("a chunk can't be decompressed: {e}")))?;
        if out.len() != CHUNK {
            return Err(bad("a chunk isn't 65536 bytes"));
        }
        plain.extend_from_slice(&out);
        if plain.len() > MAX_PLAIN {
            return Err(bad("the save is too large"));
        }
    }
    decode_plain(&plain, size, format)
}

/// Reads plain data. `size` is the used length from the file's header.
pub fn decode_plain(plain: &[u8], size: usize, format: Format) -> Result<SaveFile> {
    let used = plain.get(..size).ok_or_else(|| bad("the plain data is shorter than the header says"))?;
    let stream = match format {
        Format::Local => used,
        Format::Cloud => used.strip_prefix(&CLOUD_PREFIX).ok_or_else(|| bad("a Cloud save without the expected prefix"))?,
    };
    let n = u32_at(stream, 0)? as usize;
    if n + 16 != stream.len() {
        return Err(bad("the plain length doesn't match the table length"));
    }
    let header = [u32_at(stream, 4)?, u32_at(stream, 8)?, u32_at(stream, 12)?, u32_at(stream, 16)?];
    let mut r = Reader { b: stream, p: 20 };
    let root = r.table(0)?;
    if r.p != stream.len() {
        return Err(bad("data after the end of the table"));
    }
    Ok(SaveFile { header, root })
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl Reader<'_> {
    fn u32(&mut self) -> Result<u32> {
        let v = u32_at(self.b, self.p)?;
        self.p += 4;
        Ok(v)
    }

    /// Every tag is written at a 4-byte boundary except a table's end, which comes right after the
    /// last key however that key ended.
    fn tag(&mut self) -> Result<u32> {
        if self.p % 4 != 0 {
            let pad = 4 - self.p % 4;
            let bytes = self.b.get(self.p..self.p + pad).ok_or_else(|| bad("the file ends early"))?;
            if bytes.iter().any(|&b| b != 0) {
                let v = self.u32()?;
                return if v == TAG_CLOSE { Ok(v) } else { Err(bad(format!("unexpected tag {v}"))) };
            }
            self.p += pad;
        }
        self.u32()
    }

    fn scalar(&mut self, tag: u32) -> Result<Value> {
        match tag {
            TAG_TEXT => {
                let n = self.u32()? as usize;
                let text = self.b.get(self.p..self.p + n).ok_or_else(|| bad("text runs past the end"))?.to_vec();
                self.p += n;
                Ok(Value::Text(text))
            }
            TAG_BOOL => Ok(Value::Bool(self.u32()?)),
            TAG_NUMBER => {
                self.p = self.p.next_multiple_of(8);
                let b = self.b.get(self.p..self.p + 8).ok_or_else(|| bad("a number runs past the end"))?;
                let n = f64::from_le_bytes(b.try_into().unwrap());
                self.p += 8;
                Ok(Value::Number(n))
            }
            other => Err(bad(format!("unexpected tag {other}"))),
        }
    }

    fn table(&mut self, depth: u32) -> Result<Table> {
        if depth > MAX_DEPTH {
            return Err(bad("tables nested too deep"));
        }
        let mut out = Table::default();
        loop {
            let tag = self.tag()?;
            if tag == TAG_CLOSE {
                return Ok(out);
            }
            let value = if tag == TAG_TABLE { Value::Table(self.table(depth + 1)?) } else { self.scalar(tag)? };
            let key_tag = self.tag()?;
            let key = self.scalar(key_tag)?;
            out.0.push((key, value));
        }
    }
}

struct Writer(Vec<u8>);

impl Writer {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn tag(&mut self, t: u32) {
        while self.0.len() % 4 != 0 {
            self.0.push(0);
        }
        self.u32(t);
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Table(t) => {
                self.tag(TAG_TABLE);
                self.entries(t);
            }
            Value::Text(b) => {
                self.tag(TAG_TEXT);
                self.u32(b.len() as u32);
                self.0.extend_from_slice(b);
            }
            Value::Bool(b) => {
                self.tag(TAG_BOOL);
                self.u32(*b);
            }
            Value::Number(n) => {
                self.tag(TAG_NUMBER);
                while self.0.len() % 8 != 0 {
                    self.0.push(0);
                }
                self.0.extend_from_slice(&n.to_le_bytes());
            }
        }
    }

    fn entries(&mut self, t: &Table) {
        for (k, v) in &t.0 {
            self.value(v);
            self.value(k);
        }
        // The end of a table carries no padding, unlike every other tag.
        self.u32(TAG_CLOSE);
    }
}

/// The plain data of a file: the used bytes, with no chunk padding.
pub fn encode_plain(file: &SaveFile, format: Format) -> Vec<u8> {
    let mut w = Writer(vec![0; 20]);
    w.0[4..20].copy_from_slice(&file.header.iter().flat_map(|h| h.to_le_bytes()).collect::<Vec<u8>>());
    w.entries(&file.root);
    let mut stream = w.0;
    let n = (stream.len() - 16) as u32;
    stream[..4].copy_from_slice(&n.to_le_bytes());
    match format {
        Format::Local => stream,
        Format::Cloud => [CLOUD_PREFIX.as_slice(), stream.as_slice()].concat(),
    }
}

fn compress(chunk: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::new(6));
    z.write_all(chunk).expect("writing to a Vec");
    z.finish().expect("finishing into a Vec")
}

/// A whole file of the format. The crc, lengths and chunks are written as the game writes them.
pub fn encode_file(file: &SaveFile, format: Format) -> Vec<u8> {
    let plain = encode_plain(file, format);
    let size = plain.len();
    let mut padded = plain;
    padded.resize(size.next_multiple_of(CHUNK).max(CHUNK), 0);
    let mut chunks = Vec::new();
    for c in padded.chunks(CHUNK) {
        let z = compress(c);
        chunks.extend_from_slice(&(z.len() as u32).to_le_bytes());
        chunks.extend_from_slice(&z);
    }
    let mut body = Vec::with_capacity(chunks.len() + 16);
    body.extend_from_slice(&((chunks.len() + 8) as u32).to_le_bytes());
    body.extend_from_slice(&(size as u32).to_le_bytes());
    body.extend_from_slice(&(size as u32).to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&chunks);
    let mut out = Vec::with_capacity(body.len() + 16);
    if format == Format::Cloud {
        out.extend_from_slice(&CLOUD_MAGIC);
        out.extend_from_slice(&1u32.to_le_bytes());
    }
    out.extend_from_slice(&crc32(&body).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLANK_COMMON: &[u8] = include_bytes!("../../assets/blank-common.sav");
    const BLANK_USER: &[u8] = include_bytes!("../../assets/blank-user.sav");

    /// The plain bytes of a file: the used part, from the size in the header.
    fn plain_of(bytes: &[u8]) -> Vec<u8> {
        let at = if bytes.starts_with(&CLOUD_MAGIC) { 16 } else { 4 };
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let mut plain = Vec::new();
        let mut p = at + 16;
        while p < bytes.len() {
            let n = u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap()) as usize;
            p += 4;
            let mut out = Vec::new();
            ZlibDecoder::new(&bytes[p..p + n]).read_to_end(&mut out).unwrap();
            plain.extend(out);
            p += n;
        }
        plain.truncate(size);
        plain
    }

    fn format_of(bytes: &[u8]) -> Format {
        if bytes.starts_with(&CLOUD_MAGIC) { Format::Cloud } else { Format::Local }
    }

    /// Decode then encode gives the same plain bytes, and the same again through a second file.
    fn round_trips(bytes: &[u8]) {
        let f = decode_file(bytes).unwrap();
        let format = format_of(bytes);
        assert_eq!(encode_plain(&f, format), plain_of(bytes));
        let again = encode_file(&f, format);
        assert_eq!(plain_of(&again), plain_of(bytes));
        assert_eq!(decode_file(&again).unwrap(), f);
    }

    #[test]
    fn the_real_blank_templates_round_trip_byte_for_byte() {
        round_trips(BLANK_COMMON);
        round_trips(BLANK_USER);
    }

    #[test]
    fn a_table_ends_with_no_padding_and_a_number_lands_on_8_bytes() {
        let inner = Table(vec![(Value::text("abc"), Value::Number(2.0))]);
        let tree = Table(vec![(Value::text("reports"), Value::Table(inner))]);
        let file = SaveFile { header: [0, 1, 0, 0], root: tree };
        let plain = encode_plain(&file, Format::Local);
        let mut want = Vec::new();
        for w in [0u32, 0, 1, 0, 0] {
            want.extend_from_slice(&w.to_le_bytes());
        }
        // The nested table: tag, then value before key.
        want.extend_from_slice(&0u32.to_le_bytes());
        // Tag 9, four zero bytes to reach 8, the number.
        want.extend_from_slice(&9u32.to_le_bytes());
        while want.len() % 8 != 0 {
            want.push(0);
        }
        want.extend_from_slice(&2f64.to_le_bytes());
        want.extend_from_slice(&3u32.to_le_bytes());
        want.extend_from_slice(&3u32.to_le_bytes());
        want.extend_from_slice(b"abc");
        // The end of the nested table follows the 3 byte key at once.
        want.extend_from_slice(&1u32.to_le_bytes());
        // The next tag does get padding, then the key of the outer pair.
        while want.len() % 4 != 0 {
            want.push(0);
        }
        want.extend_from_slice(&3u32.to_le_bytes());
        want.extend_from_slice(&7u32.to_le_bytes());
        want.extend_from_slice(b"reports");
        want.extend_from_slice(&1u32.to_le_bytes());
        let n = (want.len() - 16) as u32;
        want[..4].copy_from_slice(&n.to_le_bytes());
        assert_eq!(plain, want);
        let f = decode_plain(&plain, plain.len(), Format::Local).unwrap();
        assert_eq!(f, file);
    }

    #[test]
    fn cloud_and_local_hold_the_same_table() {
        let f = decode_file(BLANK_USER).unwrap();
        let cloud = encode_file(&f, Format::Cloud);
        assert!(cloud.starts_with(&CLOUD_MAGIC));
        assert_eq!(u32::from_le_bytes(cloud[16..20].try_into().unwrap()) as usize, cloud.len() - 24);
        let local = encode_file(&f, Format::Local);
        assert_eq!(u32::from_le_bytes(local[4..8].try_into().unwrap()) as usize, local.len() - 12);
        assert_eq!(decode_file(&cloud).unwrap(), f);
        assert_eq!(&plain_of(&cloud)[8..], plain_of(&local).as_slice());
    }

    #[test]
    fn a_changed_byte_is_refused() {
        let mut b = BLANK_COMMON.to_vec();
        b[40] ^= 1;
        assert!(matches!(decode_file(&b), Err(ObsidianError::BadSave(_))));
        assert!(matches!(decode_file(&b[..10]), Err(ObsidianError::BadSave(_))));
    }

    #[test]
    fn real_saves_round_trip_when_present() {
        // Through the same search the app uses, so this also finds a Proton prefix on Linux.
        let Ok(game) = crate::game::find_game(None) else { return };
        let dirs = std::iter::once(game.saves_dir.clone()).chain(game.cloud_saves.iter().cloned());
        let files: Vec<std::path::PathBuf> = dirs.flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path())).collect();
        let mut checked = 0;
        for f in files.iter().filter(|p| p.is_file()) {
            let Ok(bytes) = std::fs::read(f) else { continue };
            // Mod data files sit beside the saves, and a save mid-write can fail the crc.
            if decode_file(&bytes).is_ok() {
                round_trips(&bytes);
                checked += 1;
            }
        }
        eprintln!("checked {checked} real save files");
    }
}
