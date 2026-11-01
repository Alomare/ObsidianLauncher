//! Timpani sound banks: a table of rows sorted by clip id, then the clips in an order of their own, each
//! starting on 4 bytes. A clip is a 68-byte header and the first part of its audio. The rest is in the
//! bundle's `.stream`, the parts one after another in the clips' order. Rebuilt with nothing new, every
//! bank of the game comes out byte for byte.

use std::collections::BTreeMap;

use crate::{ObsidianError, Result};

/// A clip's header, before its audio.
pub const CLIP_HEADER: usize = 68;
const VORBIS: u16 = 0x566F;
const PCM: u16 = 1;
const PAD: usize = 4;
const TABLE: usize = 8;
const ROW: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    pub clip: u64,
    pub off: usize,
    pub size: usize,
    pub stream_off: usize,
    pub stream_size: usize,
}

fn u32_at(raw: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(raw.get(at..at + 4)?.try_into().ok()?) as usize)
}

pub fn rows_of(raw: &[u8]) -> Result<Vec<Row>> {
    let bad = || ObsidianError::BadBank("its table runs past its end".into());
    let count = u32_at(raw, 0).ok_or_else(bad)?;
    if count.checked_mul(ROW).and_then(|n| n.checked_add(TABLE)).is_none_or(|end| end > raw.len()) {
        return Err(bad());
    }
    (0..count)
        .map(|i| {
            let at = TABLE + ROW * i;
            let clip = u64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
            let (off, size) = (u32_at(raw, at + 8).ok_or_else(bad)?, u32_at(raw, at + 12).ok_or_else(bad)?);
            let (stream_off, stream_size) = (u32_at(raw, at + 16).ok_or_else(bad)?, u32_at(raw, at + 20).ok_or_else(bad)?);
            if off + size > raw.len() {
                return Err(bad());
            }
            Ok(Row { clip, off, size, stream_off, stream_size })
        })
        .collect()
}

/// The bank and its stream with some clips in place of the game's: `new` is {clip: (bytes in the bank,
/// header included; bytes in the stream)}. Everything else stays where the bank's layout puts it.
pub fn rebuild(raw: &[u8], stream: &[u8], new: &BTreeMap<u64, (&[u8], &[u8])>) -> Result<(Vec<u8>, Vec<u8>)> {
    let rows = rows_of(raw)?;
    let mut out = Vec::with_capacity(raw.len());
    out.extend_from_slice(&raw[..TABLE]);
    out.resize(TABLE + ROW * rows.len(), 0);
    let mut out_stream = Vec::with_capacity(stream.len());
    let mut in_layout = rows.clone();
    in_layout.sort_by_key(|r| r.off);
    let mut placed = BTreeMap::new();
    for r in &in_layout {
        out.resize(out.len().next_multiple_of(PAD), 0);
        let (kept, streamed) = match new.get(&r.clip) {
            Some(n) => *n,
            None => {
                let part = r.stream_off.checked_add(r.stream_size).and_then(|end| stream.get(r.stream_off..end));
                (&raw[r.off..r.off + r.size], part.ok_or_else(|| ObsidianError::BadBank("a clip runs past the end of its stream".into()))?)
            }
        };
        placed.insert(r.clip, (out.len(), kept.len(), out_stream.len(), streamed.len()));
        out.extend_from_slice(kept);
        out_stream.extend_from_slice(streamed);
    }
    for (i, r) in rows.iter().enumerate() {
        let (off, size, stream_off, stream_size) = placed[&r.clip];
        let at = TABLE + ROW * i;
        out[at..at + 8].copy_from_slice(&r.clip.to_le_bytes());
        for (n, v) in [off, size, stream_off, stream_size].into_iter().enumerate() {
            let v = u32::try_from(v).map_err(|_| ObsidianError::BadBank("it would pass 4 GB".into()))?;
            out[at + 8 + n * 4..at + 12 + n * 4].copy_from_slice(&v.to_le_bytes());
        }
    }
    Ok((out, out_stream))
}

/// What's wrong with the bytes a clip puts in the bank, if anything.
pub fn check_clip(data: &[u8]) -> std::result::Result<(), String> {
    if data.len() < CLIP_HEADER {
        return Err(format!("it has {} bytes, and a clip's header alone has {CLIP_HEADER}", data.len()));
    }
    let tag = u16::from_le_bytes(data[48..50].try_into().unwrap());
    if tag != VORBIS && tag != PCM {
        return Err(format!("its header names codec {tag:#x}, which isn't Ogg Vorbis or PCM"));
    }
    let kept = u32_at(data, 4).unwrap_or(0);
    if kept != data.len() - CLIP_HEADER {
        return Err(format!("its header says {kept} bytes of audio in the bank, and the file has {}", data.len() - CLIP_HEADER));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A clip as a bank holds it: the header, then `audio`.
    pub(crate) fn clip(audio: &[u8]) -> Vec<u8> {
        let mut c = vec![0u8; CLIP_HEADER];
        c[0..4].copy_from_slice(&(CLIP_HEADER as u32).to_le_bytes());
        c[4..8].copy_from_slice(&(audio.len() as u32).to_le_bytes());
        c[48..50].copy_from_slice(&PCM.to_le_bytes());
        c.extend_from_slice(audio);
        c
    }

    /// A bank and its stream, with the clips laid out in the order given and the rows sorted by id.
    pub(crate) fn make(clips: &[(u64, &[u8], &[u8])]) -> (Vec<u8>, Vec<u8>) {
        let mut rows: Vec<_> = clips.iter().map(|c| c.0).collect();
        rows.sort_unstable();
        let mut out = (clips.len() as u32).to_le_bytes().to_vec();
        out.resize(TABLE + ROW * clips.len(), 0);
        let mut stream = Vec::new();
        let mut placed = BTreeMap::new();
        for (id, audio, streamed) in clips {
            out.resize(out.len().next_multiple_of(PAD), 0);
            let c = clip(audio);
            placed.insert(*id, (out.len() as u32, c.len() as u32, stream.len() as u32, streamed.len() as u32));
            out.extend_from_slice(&c);
            stream.extend_from_slice(streamed);
        }
        for (i, id) in rows.iter().enumerate() {
            let (a, b, c, d) = placed[id];
            let at = TABLE + ROW * i;
            out[at..at + 8].copy_from_slice(&id.to_le_bytes());
            for (n, v) in [a, b, c, d].into_iter().enumerate() {
                out[at + 8 + n * 4..at + 12 + n * 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        (out, stream)
    }

    #[test]
    fn a_bank_rebuilt_with_nothing_new_is_the_same_bytes() {
        let (raw, stream) = make(&[(9, b"nine!", b"NINE"), (2, b"two", b""), (5, b"fivefive", b"FIVE")]);
        let (again, again_stream) = rebuild(&raw, &stream, &BTreeMap::new()).unwrap();
        assert_eq!((again, again_stream), (raw, stream));
    }

    #[test]
    fn a_new_clip_takes_its_place_and_the_rest_moves_with_it() {
        let (raw, stream) = make(&[(9, b"nine!", b"NINE"), (2, b"two", b""), (5, b"fivefive", b"FIVE")]);
        let new_clip = clip(b"a longer two");
        let (out, out_stream) = rebuild(&raw, &stream, &BTreeMap::from([(2, (new_clip.as_slice(), b"TWO".as_slice()))])).unwrap();
        assert_eq!(out_stream, b"NINETWOFIVE", "the stream follows the clips' order, with nothing between");
        let rows = rows_of(&out).unwrap();
        assert_eq!(rows.iter().map(|r| r.clip).collect::<Vec<_>>(), [2, 5, 9], "the rows stay sorted by id");
        let of = |id| *rows.iter().find(|r| r.clip == id).unwrap();
        assert!(of(9).off < of(2).off && of(2).off < of(5).off, "the clips keep their order");
        assert_eq!(&out[of(2).off..of(2).off + of(2).size], new_clip.as_slice());
        assert_eq!(&out[of(5).off + CLIP_HEADER..of(5).off + of(5).size], b"fivefive");
        assert!(rows.iter().all(|r| r.off % PAD == 0), "each clip starts on 4 bytes");
        assert_eq!((of(5).stream_off, of(5).stream_size), (7, 4));
    }

    #[test]
    fn a_bank_that_doesnt_add_up_is_refused() {
        assert!(matches!(rows_of(&[200, 0, 0, 0, 0, 0, 0, 0]), Err(ObsidianError::BadBank(_))));
        let (raw, _) = make(&[(1, b"a", b"STREAM")]);
        assert!(matches!(rebuild(&raw, b"", &BTreeMap::new()), Err(ObsidianError::BadBank(_))));
    }

    #[test]
    fn a_clips_file_is_checked() {
        assert_eq!(check_clip(&clip(b"audio")), Ok(()));
        assert!(check_clip(b"short").unwrap_err().contains("header alone"));
        let mut wrong = clip(b"audio");
        wrong[48] = 9;
        assert!(check_clip(&wrong).unwrap_err().contains("codec"));
        let mut wrong = clip(b"audio");
        wrong[4] = 99;
        assert!(check_clip(&wrong).unwrap_err().contains("99 bytes"));
    }
}
