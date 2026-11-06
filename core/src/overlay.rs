//! Sound clips deployed as an overlay, so a sound mod's patch is about the size of its new audio.
//!
//! A patch replaces a resource whole, every language at once, and a Timpani bank holds every clip of
//! its part of the game in each of up to 9 languages: patching the game's bank for one footstep wrote up
//! to 1.8 GB per bank. The game's `timpani_master` is its one list of sounds (570 KB, one language, in
//! the boot bundle), and a clip record there names its clip by id alone. The engine finds that id in
//! any loaded bank. So the master names new ids where it named the clips a mod replaces, and the clips
//! go under those ids into a small bank of Obsidian's beside each bank they replace, in every bundle that
//! holds it, so they load with it. The game's banks stay as they are. Not in the boot bundle: it loads
//! before the game sets the language, so a resource there takes language 0 in every language (round J,
//! 2026-11-06: an English-only beep played in French too), and the game keeps nothing with languages
//! there. A language whose clips are the same as those of language 0 gets no variant of its own: the
//! engine falls back to language 0.

use std::collections::{BTreeMap, BTreeSet};

use crate::hash::hash_name;
use crate::{ObsidianError, Result};

/// The first word of a master.
pub const MAGIC: u32 = 0x6191_4940;
/// Obsidian's banks of new clips: this, then `/<bank id>` of the bank they replace.
pub const BANK_NAME: &str = "obsidian/timpani/clips";
/// The bytes of a clip record: its kind (0), then the clip's id at `CLIP_AT`.
const RECORD: usize = 72;
const CLIP_AT: usize = 16;
const PAD: usize = 4;
const ROW: usize = 24;

fn u32_at(raw: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(raw.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// Where each clip record of a master names its clip: (offset of the id, the clip), in record order.
/// The master starts with its magic and the offsets of 7 sections: the sounds are the third, a count,
/// that many offsets from the section's start, then the records, which end where the events start.
pub fn clip_places(master: &[u8]) -> Result<Vec<(usize, u64)>> {
    let bad = |why: &str| ObsidianError::BadMaster(why.to_string());
    if u32_at(master, 0) != Some(MAGIC as usize) {
        return Err(bad("it doesn't start as a master does"));
    }
    let sounds = u32_at(master, 12).ok_or_else(|| bad("it is cut short"))?;
    let events = u32_at(master, 16).ok_or_else(|| bad("it is cut short"))?;
    let count = u32_at(master, sounds).ok_or_else(|| bad("its list of sounds runs past its end"))?;
    if events > master.len() || count.checked_mul(4).and_then(|n| n.checked_add(sounds + 4)).is_none_or(|end| end > events) {
        return Err(bad("its list of sounds runs past its end"));
    }
    let mut out = Vec::new();
    for i in 0..count {
        let at = sounds + u32_at(master, sounds + 4 + 4 * i).unwrap();
        let kind = u32_at(master, at).ok_or_else(|| bad("a sound runs past its end"))?;
        if kind == 0 {
            if at + RECORD > events {
                return Err(bad("a clip record runs past the list of sounds"));
            }
            out.push((at + CLIP_AT, u64::from_le_bytes(master[at + CLIP_AT..at + CLIP_AT + 8].try_into().unwrap())));
        }
    }
    Ok(out)
}

/// The master with the clips of `map` named by their new ids, every record that names one.
pub fn remap(master: &[u8], map: &BTreeMap<u64, u64>) -> Result<Vec<u8>> {
    let mut out = master.to_vec();
    for (at, clip) in clip_places(master)? {
        if let Some(new) = map.get(&clip) {
            out[at..at + 8].copy_from_slice(&new.to_le_bytes());
        }
    }
    Ok(out)
}

/// A new id for each clip, the same every deploy: the hash of `obsidian/clip/<id>`, then of
/// `obsidian/clip/<id>/<n>` while it is taken (by the game or by another new id).
pub fn new_ids(clips: &BTreeSet<u64>, taken: &BTreeSet<u64>) -> BTreeMap<u64, u64> {
    let mut used: BTreeSet<u64> = taken.iter().chain(clips).copied().collect();
    let mut out = BTreeMap::new();
    for &clip in clips {
        let mut id = hash_name(&format!("obsidian/clip/{clip:016x}"));
        let mut n = 1;
        while used.contains(&id) {
            id = hash_name(&format!("obsidian/clip/{clip:016x}/{n}"));
            n += 1;
        }
        used.insert(id);
        out.insert(clip, id);
    }
    out
}

/// A bank of these clips, each (id, bytes in the bank with the 68-byte header, bytes in the stream):
/// the rows sorted by id, the clips laid out in that order on 4 bytes, their stream parts one after
/// another in the same order. Returns (bank, stream).
pub fn make_bank(clips: &[(u64, &[u8], &[u8])]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut sorted: Vec<&(u64, &[u8], &[u8])> = clips.iter().collect();
    sorted.sort_by_key(|c| c.0);
    let mut out = (sorted.len() as u32).to_le_bytes().to_vec();
    out.resize(8 + ROW * sorted.len(), 0);
    let mut stream = Vec::new();
    let too_big = || ObsidianError::BadBank("it would pass 4 GB".into());
    for (i, (id, data, streamed)) in sorted.iter().enumerate() {
        out.resize(out.len().next_multiple_of(PAD), 0);
        let row = [out.len(), data.len(), stream.len(), streamed.len()];
        let at = 8 + ROW * i;
        out[at..at + 8].copy_from_slice(&id.to_le_bytes());
        for (n, v) in row.into_iter().enumerate() {
            let v = u32::try_from(v).map_err(|_| too_big())?;
            out[at + 8 + 4 * n..at + 12 + 4 * n].copy_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(data);
        stream.extend_from_slice(streamed);
    }
    Ok((out, stream))
}

/// A clip as a bank holds it: the bytes in the bank, header first, and those in the stream.
pub type ClipBytes = (Vec<u8>, Vec<u8>);

/// The languages of Obsidian's bank and their clips by new id. `heard` gives, for each language, the
/// clip every replaced clip plays in it. Language 0 is always there; another language only where some
/// clip differs from language 0's.
pub fn languages(heard: &BTreeMap<u32, BTreeMap<u64, ClipBytes>>) -> Vec<u32> {
    let first = heard.get(&0);
    heard.iter().filter(|(lang, clips)| **lang == 0 || Some(*clips) != first).map(|(lang, _)| *lang).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A master with these sounds: `Some(clip)` is a clip record, `None` a random pick (another kind).
    pub(crate) fn master(sounds: &[Option<u64>]) -> Vec<u8> {
        let mut records = Vec::new();
        let mut offsets = Vec::new();
        let table = 4 + 4 * sounds.len();
        for s in sounds {
            offsets.push((table + records.len()) as u32);
            let mut r = match s {
                Some(clip) => {
                    let mut r = vec![0u8; RECORD];
                    r[CLIP_AT..CLIP_AT + 8].copy_from_slice(&clip.to_le_bytes());
                    r[24..28].copy_from_slice(&1.0f32.to_le_bytes());
                    r
                }
                None => {
                    let mut r = vec![0u8; 32];
                    r[0..4].copy_from_slice(&1u32.to_le_bytes());
                    r
                }
            };
            records.append(&mut r);
        }
        let sounds_at = 32 + 8;
        let mut section = (sounds.len() as u32).to_le_bytes().to_vec();
        for o in offsets {
            section.extend(o.to_le_bytes());
        }
        section.extend(records);
        let events_at = sounds_at + section.len();
        let events = 0u32.to_le_bytes();
        let end = events_at + events.len();
        let mut m = MAGIC.to_le_bytes().to_vec();
        for at in [32, 32, sounds_at, events_at, end, end, end] {
            m.extend((at as u32).to_le_bytes());
        }
        m.extend([7u8; 8]); // the buses and the second section, which the overlay never reads
        m.extend(section);
        m.extend(events);
        m
    }

    #[test]
    fn clip_records_are_found_and_named_anew() {
        let m = master(&[Some(9), None, Some(5), Some(9)]);
        let places = clip_places(&m).unwrap();
        assert_eq!(places.iter().map(|p| p.1).collect::<Vec<_>>(), [9, 5, 9]);
        let again = remap(&m, &BTreeMap::from([(9, 99)])).unwrap();
        assert_eq!(clip_places(&again).unwrap().iter().map(|p| p.1).collect::<Vec<_>>(), [99, 5, 99]);
        let changed: Vec<usize> = (0..m.len()).filter(|&i| m[i] != again[i]).collect();
        assert!(changed.iter().all(|&i| places.iter().any(|&(at, _)| i >= at && i < at + 8)), "only the ids change");
    }

    #[test]
    fn a_master_that_doesnt_add_up_is_refused() {
        let mut m = master(&[Some(1)]);
        assert!(matches!(clip_places(&m[..20]), Err(ObsidianError::BadMaster(_))));
        m[0] = 0;
        assert!(matches!(clip_places(&m), Err(ObsidianError::BadMaster(_))));
        let mut m = master(&[Some(1)]);
        m[40..44].copy_from_slice(&5000u32.to_le_bytes());
        assert!(matches!(clip_places(&m), Err(ObsidianError::BadMaster(_))), "a count past the end");
    }

    #[test]
    fn new_ids_stay_the_same_and_step_past_taken_ones() {
        let clips = BTreeSet::from([5, 9]);
        let a = new_ids(&clips, &BTreeSet::new());
        assert_eq!(a, new_ids(&clips, &BTreeSet::new()), "the same every deploy");
        assert_eq!(a[&5], hash_name("obsidian/clip/0000000000000005"));
        let b = new_ids(&clips, &BTreeSet::from([a[&5]]));
        assert_eq!(b[&5], hash_name("obsidian/clip/0000000000000005/1"));
        assert_eq!(b[&9], a[&9]);
    }

    #[test]
    fn a_bank_made_here_reads_back_as_the_game_reads_banks() {
        let (bank, stream) = make_bank(&[(9, b"nine!", b"NINE"), (2, b"two", b""), (5, b"fivefive", b"FIVE")]).unwrap();
        let rows = crate::bank::rows_of(&bank).unwrap();
        assert_eq!(rows.iter().map(|r| r.clip).collect::<Vec<_>>(), [2, 5, 9]);
        assert!(rows.iter().all(|r| r.off % PAD == 0));
        let of = |id| *rows.iter().find(|r| r.clip == id).unwrap();
        assert_eq!(&bank[of(5).off..of(5).off + of(5).size], b"fivefive");
        assert_eq!(&stream[of(9).stream_off..of(9).stream_off + of(9).stream_size], b"NINE");
        assert_eq!(stream, b"FIVENINE", "stream parts in the clips' order");
        let (again, again_stream) = crate::bank::rebuild(&bank, &stream, &BTreeMap::new()).unwrap();
        assert_eq!((again, again_stream), (bank, stream), "the layout the game's banks have");
    }

    #[test]
    fn a_language_like_the_first_gets_no_variant() {
        let clip = |b: &[u8]| (b.to_vec(), Vec::new());
        let heard = BTreeMap::from([
            (0, BTreeMap::from([(1, clip(b"a")), (2, clip(b"b"))])),
            (0x100, BTreeMap::from([(1, clip(b"a")), (2, clip(b"b"))])),
            (0x4000, BTreeMap::from([(1, clip(b"a")), (2, clip(b"de"))])),
        ]);
        assert_eq!(languages(&heard), [0, 0x4000]);
    }
}
