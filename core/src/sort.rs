//! Sort Mods: the load order put in line with what each mod asks for (`load` in the manifest).
//!
//! Mods that ask for nothing keep their order, and only the mods a request needs move: the fewest
//! moves first. Between orders with as many moves, two mods that replace the same files go the way
//! that overwrites less (the mod with fewer files goes later, so it isn't lost entirely), and then
//! the order nearest the current one stands. Requests that can't all hold aren't guessed at: the
//! mods involved stay where they were and are reported.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;

use crate::Result;
use crate::library::{Library, Profile};
use crate::manifest::{LoadRules, Manifest, Payload, Position, load_manifest};

/// One mod as the sorter sees it, in the current order.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub id: String,
    pub rules: LoadRules,
    /// Replaces game files: the mods `before_patches` and `after_patches` count.
    pub patch: bool,
    /// The files a deploy writes for it, by key, with the name to show.
    pub files: BTreeMap<String, String>,
    /// Stays where it is: it sits in a locked folder, or can't be read.
    pub fixed: bool,
    /// No mod may go right after this one: the next mod is in the same locked folder.
    pub sealed_after: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Before,
    After,
    BeforePatches,
    AfterPatches,
    First,
    Last,
}

/// One request: `owner` asks for it, `other` is the mod named by before or after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rule {
    #[serde(rename = "mod")]
    pub owner: String,
    pub kind: RuleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
}

/// The file rule deciding between two mods that replace the same files: `later` when the moved mod
/// goes after `other`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tie {
    pub other: String,
    pub later: bool,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Move {
    pub id: String,
    pub from: usize,
    pub to: usize,
    /// The requests that put it out of place.
    pub rules: Vec<Rule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tie: Option<Tie>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashKind {
    BothFirst,
    BothLast,
    Cycle,
    /// A request needs a mod in a locked folder to move, or a mod to go inside one.
    Locked,
}

/// Requests that can't all hold. The mods stay where they were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Clash {
    pub kind: ClashKind,
    pub mods: Vec<String>,
    pub rules: Vec<Rule>,
}

/// A request naming a mod that isn't in the profile: it is skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    #[serde(rename = "mod")]
    pub owner: String,
    pub missing: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SortPlan {
    pub order: Vec<String>,
    pub moves: Vec<Move>,
    pub conflicts: Vec<Clash>,
    pub notes: Vec<Note>,
}

/// A set of mods by index.
#[derive(Clone, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(n: usize) -> Bits {
        Bits(vec![0; n.div_ceil(64).max(1)])
    }
    fn get(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }
    fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }
}

/// A request with indexes: `other` is the index of the mod named.
#[derive(Clone, Copy)]
struct RawRule {
    owner: usize,
    kind: RuleKind,
    other: Option<usize>,
}

struct Graph {
    /// `succ[u]`: the mods that must come after `u`, each with the request that says so.
    succ: Vec<Vec<(usize, usize)>>,
    /// `reach[u]`: every mod that must come after `u`, directly or through others.
    reach: Vec<Bits>,
}

fn edges_of(r: &RawRule, entries: &[Entry]) -> Vec<(usize, usize)> {
    let n = entries.len();
    let i = r.owner;
    let others = (0..n).filter(move |&j| j != i);
    match r.kind {
        RuleKind::Before => vec![(i, r.other.unwrap())],
        RuleKind::After => vec![(r.other.unwrap(), i)],
        RuleKind::BeforePatches => others.filter(|&j| entries[j].patch).map(|j| (i, j)).collect(),
        RuleKind::AfterPatches => others.filter(|&j| entries[j].patch).map(|j| (j, i)).collect(),
        RuleKind::First => others.map(|j| (i, j)).collect(),
        RuleKind::Last => others.map(|j| (j, i)).collect(),
    }
}

fn graph(entries: &[Entry], rules: &[RawRule], pinned: &[bool]) -> Graph {
    let n = entries.len();
    let mut succ: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (k, r) in rules.iter().enumerate() {
        if pinned[r.owner] {
            continue;
        }
        for (u, v) in edges_of(r, entries) {
            if seen.insert((u, v)) {
                succ[u].push((v, k));
            }
        }
    }
    let mut reach = vec![Bits::new(n); n];
    for (s, row) in reach.iter_mut().enumerate() {
        let mut stack: Vec<usize> = succ[s].iter().map(|e| e.0).collect();
        while let Some(u) = stack.pop() {
            if row.get(u) {
                continue;
            }
            row.set(u);
            stack.extend(succ[u].iter().map(|e| e.0).filter(|&v| !row.get(v)));
        }
    }
    Graph { succ, reach }
}

fn rule_out(r: &RawRule, entries: &[Entry]) -> Rule {
    Rule { owner: entries[r.owner].id.clone(), kind: r.kind, other: r.other.map(|o| entries[o].id.clone()) }
}

/// The requests of every mod, and the names they give that aren't there.
fn collect_rules(entries: &[Entry]) -> (Vec<RawRule>, Vec<Note>) {
    let index: HashMap<&str, usize> = entries.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
    let (mut rules, mut notes) = (Vec::new(), Vec::new());
    for (i, e) in entries.iter().enumerate() {
        for (kind, ids) in [(RuleKind::Before, &e.rules.before), (RuleKind::After, &e.rules.after)] {
            for id in ids {
                match index.get(id.as_str()) {
                    Some(&j) if j != i => rules.push(RawRule { owner: i, kind, other: Some(j) }),
                    Some(_) => {}
                    None => notes.push(Note { owner: e.id.clone(), missing: id.clone() }),
                }
            }
        }
        if e.rules.before_patches {
            rules.push(RawRule { owner: i, kind: RuleKind::BeforePatches, other: None });
        }
        if e.rules.after_patches {
            rules.push(RawRule { owner: i, kind: RuleKind::AfterPatches, other: None });
        }
        match e.rules.position {
            Some(Position::First) => rules.push(RawRule { owner: i, kind: RuleKind::First, other: None }),
            Some(Position::Last) => rules.push(RawRule { owner: i, kind: RuleKind::Last, other: None }),
            None => {}
        }
    }
    (rules, notes)
}

/// How much `a` going later than `b` is preferred over the other way: the shared files, positive when
/// `a` has fewer files than `b`, so going later it overwrites a smaller part of `b`.
fn preference(a: &Entry, b: &Entry) -> i64 {
    let shared = a.files.keys().filter(|k| b.files.contains_key(*k)).count() as i64;
    match a.files.len().cmp(&b.files.len()) {
        std::cmp::Ordering::Less => shared,
        std::cmp::Ordering::Greater => -shared,
        std::cmp::Ordering::Equal => 0,
    }
}

/// Where the moved mods go, among the kept mods in their order. `Err` names a mod with nowhere to go
/// outside a locked folder.
fn place(entries: &[Entry], g: &Graph, kept: &[bool], prefs: &[Vec<i64>]) -> std::result::Result<Vec<usize>, usize> {
    let n = entries.len();
    let mut seq: Vec<usize> = (0..n).filter(|&i| kept[i]).collect();
    let mut todo: Vec<usize> = (0..n).filter(|&i| !kept[i]).collect();
    while !todo.is_empty() {
        // The earliest mod no other waiting mod has to precede.
        let k = (0..todo.len()).find(|&k| !todo.iter().any(|&x| x != todo[k] && g.reach[x].get(todo[k]))).unwrap_or(0);
        let m = todo.remove(k);
        let lo = seq.iter().rposition(|&x| g.reach[x].get(m)).map_or(0, |i| i + 1);
        let hi = seq.iter().position(|&y| g.reach[m].get(y)).unwrap_or(seq.len());
        let natural = seq.iter().filter(|&&x| x < m).count();
        let mut best: Option<(i64, usize)> = None;
        for s in lo..=hi.max(lo) {
            if s > 0 && entries[seq[s - 1]].sealed_after {
                continue;
            }
            let file: i64 = seq.iter().enumerate().map(|(at, &z)| if at < s { prefs[m][z] } else { -prefs[m][z] }).sum();
            let score = file * (4 * n as i64 + 4) - (s as i64 - natural as i64).abs() * 2 - i64::from(s > natural);
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, s));
            }
        }
        let Some((_, s)) = best else { return Err(m) };
        seq.insert(s, m);
    }
    Ok(seq)
}

fn file_score(order: &[usize], prefs: &[Vec<i64>]) -> i64 {
    let mut total = 0;
    for (a, &x) in order.iter().enumerate() {
        for &y in &order[a + 1..] {
            total += prefs[y][x];
        }
    }
    total
}

fn displacement(order: &[usize]) -> usize {
    order.iter().enumerate().map(|(at, &i)| at.abs_diff(i)).sum()
}

/// The largest set of mods among `pool` with no two the wrong way round (`inv`): the mods that keep
/// their place. With few mods involved, every such set, so the file rule can choose; with many, one.
fn keep_sets(pool: &[usize], inv: &dyn Fn(usize, usize) -> bool) -> Vec<Vec<usize>> {
    if pool.len() <= 16 {
        let k = pool.len();
        let mut clash = vec![0u32; k];
        for a in 0..k {
            for b in 0..k {
                if a != b && inv(pool[a], pool[b]) {
                    clash[a] |= 1 << b;
                }
            }
        }
        let mut best = 0;
        let mut sets: Vec<u32> = Vec::new();
        for mask in 0u32..(1 << k) {
            let size = mask.count_ones();
            if size < best || (0..k).any(|a| mask >> a & 1 == 1 && clash[a] & mask != 0) {
                continue;
            }
            if size > best {
                best = size;
                sets.clear();
            }
            if sets.len() < 256 {
                sets.push(mask);
            }
        }
        return sets.into_iter().map(|m| (0..k).filter(|&a| m >> a & 1 == 1).map(|a| pool[a]).collect()).collect();
    }
    vec![dilworth(pool, inv)]
}

/// Dilworth: the mods out of order form a partial order (must come first, yet sit later), and the most
/// that can stay is the largest antichain, read from a maximum matching (König).
fn dilworth(pool: &[usize], inv: &dyn Fn(usize, usize) -> bool) -> Vec<usize> {
    let k = pool.len();
    let less = |a: usize, b: usize| inv(pool[a], pool[b]) && pool[b] < pool[a];
    let adj: Vec<Vec<usize>> = (0..k).map(|a| (0..k).filter(|&b| a != b && less(a, b)).collect()).collect();
    let mut right: Vec<Option<usize>> = vec![None; k];
    let mut left: Vec<Option<usize>> = vec![None; k];
    fn augment(a: usize, adj: &[Vec<usize>], seen: &mut [bool], left: &mut [Option<usize>], right: &mut [Option<usize>]) -> bool {
        for &b in &adj[a] {
            if seen[b] {
                continue;
            }
            seen[b] = true;
            if right[b].is_none_or(|a2| augment(a2, adj, seen, left, right)) {
                right[b] = Some(a);
                left[a] = Some(b);
                return true;
            }
        }
        false
    }
    for a in 0..k {
        let mut seen = vec![false; k];
        augment(a, &adj, &mut seen, &mut left, &mut right);
    }
    let (mut zl, mut zr) = (vec![false; k], vec![false; k]);
    let mut stack: Vec<usize> = (0..k).filter(|&a| left[a].is_none()).collect();
    for &a in &stack {
        zl[a] = true;
    }
    while let Some(a) = stack.pop() {
        for &b in &adj[a] {
            if Some(b) != left[a] && !zr[b] {
                zr[b] = true;
                if let Some(a2) = right[b].filter(|&a2| !zl[a2]) {
                    zl[a2] = true;
                    stack.push(a2);
                }
            }
        }
    }
    (0..k).filter(|&x| zl[x] && !zr[x]).map(|x| pool[x]).collect()
}

/// The best order for the requests in `g`, with `fixed` mods kept: fewest moves, then the file rule
/// (when `prefs` has any), then the least displacement.
/// `asks[x]`: the mod has requests of its own. Between equal orders, the mod that asked is the one to move.
fn solve(entries: &[Entry], g: &Graph, fixed: &[bool], prefs: &[Vec<i64>], asks: &[bool]) -> std::result::Result<(Vec<usize>, Vec<bool>), usize> {
    let n = entries.len();
    // `inv(a, b)`: the two are the wrong way round now.
    let inv = |a: usize, b: usize| if a < b { g.reach[b].get(a) } else { g.reach[a].get(b) };
    let must_move: Vec<bool> = (0..n).map(|x| !fixed[x] && (0..n).any(|y| fixed[y] && inv(x, y))).collect();
    let free: Vec<usize> = (0..n).filter(|&x| !fixed[x] && !must_move[x]).collect();
    let pool: Vec<usize> = free.iter().copied().filter(|&x| free.iter().any(|&y| y != x && inv(x, y))).collect();
    // The score (file rule, closeness, the mods that asked moving), the order and the kept mods.
    type Best = ((i64, i64, i64), Vec<usize>, Vec<bool>);
    let mut best: Option<Best> = None;
    let mut failed = None;
    for keep in keep_sets(&pool, &inv) {
        let kept: Vec<bool> = (0..n).map(|x| fixed[x] || (free.contains(&x) && (!pool.contains(&x) || keep.contains(&x)))).collect();
        match place(entries, g, &kept, prefs) {
            Ok(order) => {
                let quiet_moved = (0..n).filter(|&x| !kept[x] && !asks[x]).count() as i64;
                let score = (file_score(&order, prefs), -(displacement(&order) as i64), -quiet_moved);
                if best.as_ref().is_none_or(|(b, _, _)| score > *b) {
                    best = Some((score, order, kept));
                }
            }
            Err(m) => failed = failed.or(Some(m)),
        }
    }
    match (best, failed) {
        (Some((_, order, kept)), _) => Ok((order, kept)),
        (None, Some(m)) => Err(m),
        (None, None) => Ok(((0..n).collect(), vec![true; n])),
    }
}

/// The path of requests from `a` to `b`: the mods on it and the requests.
fn path(g: &Graph, a: usize, b: usize) -> (Vec<usize>, Vec<usize>) {
    let mut parent: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut queue = std::collections::VecDeque::from([a]);
    while let Some(u) = queue.pop_front() {
        if u == b {
            break;
        }
        for &(v, k) in &g.succ[u] {
            if v != a && !parent.contains_key(&v) {
                parent.insert(v, (u, k));
                queue.push_back(v);
            }
        }
    }
    let (mut nodes, mut rules) = (vec![b], Vec::new());
    let mut at = b;
    while let Some(&(u, k)) = parent.get(&at) {
        rules.push(k);
        nodes.push(u);
        at = u;
        if at == a {
            break;
        }
    }
    (nodes, rules)
}

pub fn sort(entries: &[Entry]) -> SortPlan {
    let n = entries.len();
    let (rules, notes) = collect_rules(entries);
    let mut pinned = vec![false; n];
    let mut conflicts: Vec<Clash> = Vec::new();
    let clash = |kind: ClashKind, mut mods: Vec<usize>, mut ks: Vec<usize>| {
        mods.sort_unstable();
        mods.dedup();
        ks.sort_unstable_by_key(|&k| (rules[k].owner, rules[k].kind));
        ks.dedup();
        Clash { kind, mods: mods.iter().map(|&i| entries[i].id.clone()).collect(), rules: ks.iter().map(|&k| rule_out(&rules[k], entries)).collect() }
    };
    let asks: Vec<bool> = (0..n).map(|i| rules.iter().any(|r| r.owner == i)).collect();
    let (g, fixed, (order_with, kept), (order_without, _)) = loop {
        let g = graph(entries, &rules, &pinned);
        // Requests in a circle, such as two mods that both ask to load last.
        let mut found = false;
        let mut done = vec![false; n];
        for u in 0..n {
            if done[u] || !g.reach[u].get(u) {
                continue;
            }
            let members: Vec<usize> = (0..n).filter(|&v| v == u || (g.reach[u].get(v) && g.reach[v].get(u))).collect();
            let inside: Vec<usize> = members
                .iter()
                .flat_map(|&a| g.succ[a].iter().filter(|(b, _)| members.contains(b)).map(|&(_, k)| k))
                .collect();
            let kinds: BTreeSet<RuleKind> = inside.iter().map(|&k| rules[k].kind).collect();
            let kind = match kinds.iter().copied().collect::<Vec<_>>()[..] {
                [RuleKind::Last] => ClashKind::BothLast,
                [RuleKind::First] => ClashKind::BothFirst,
                _ => ClashKind::Cycle,
            };
            for &m in &members {
                done[m] = true;
                pinned[m] = true;
            }
            conflicts.push(clash(kind, members, inside));
            found = true;
        }
        if found {
            continue;
        }
        let fixed: Vec<bool> = (0..n).map(|i| entries[i].fixed || pinned[i]).collect();
        // Two mods that stay put, the wrong way round for the requests between them.
        if let Some((a, b)) = (0..n).flat_map(|a| (0..a).map(move |b| (a, b))).find(|&(a, b)| fixed[a] && fixed[b] && g.reach[a].get(b)) {
            let (nodes, ks) = path(&g, a, b);
            for &k in &ks {
                pinned[rules[k].owner] = true;
            }
            conflicts.push(clash(ClashKind::Locked, nodes, ks));
            continue;
        }
        let none = vec![vec![0i64; n]; n];
        let prefs: Vec<Vec<i64>> = (0..n).map(|a| (0..n).map(|b| if a == b { 0 } else { preference(&entries[a], &entries[b]) }).collect()).collect();
        match (solve(entries, &g, &fixed, &prefs, &asks), solve(entries, &g, &fixed, &none, &asks)) {
            (Ok(with), Ok(without)) => break (g, fixed, with, without),
            (Err(m), _) | (_, Err(m)) => {
                // Nowhere to go outside a locked folder: the mod keeps its place.
                let ks: Vec<usize> = (0..rules.len()).filter(|&k| !pinned[rules[k].owner] && (rules[k].owner == m || rules[k].other == Some(m))).collect();
                pinned[m] = true;
                conflicts.push(clash(ClashKind::Locked, vec![m], ks));
            }
        }
    };

    let at = |order: &[usize]| {
        let mut p = vec![0; n];
        for (i, &x) in order.iter().enumerate() {
            p[x] = i;
        }
        p
    };
    let (new_at, alt_at) = (at(&order_with), at(&order_without));
    let moved: Vec<usize> = (0..n).filter(|&i| !kept[i]).collect();
    let mut moves = Vec::new();
    for &m in &moved {
        let incident: Vec<(usize, usize, usize)> = (0..n)
            .flat_map(|u| g.succ[u].iter().map(move |&(v, k)| (u, v, k)))
            .filter(|&(u, v, _)| u == m || v == m)
            .collect();
        let mut ks: Vec<usize> = incident.iter().filter(|&&(u, v, _)| v < u).map(|e| e.2).collect();
        if ks.is_empty() {
            ks = incident.iter().filter(|&&(u, v, _)| (new_at[u] < new_at[v]) != (u < v)).map(|e| e.2).collect();
        }
        if ks.is_empty() {
            ks = incident.iter().map(|e| e.2).collect();
        }
        ks.sort_unstable_by_key(|&k| (rules[k].owner != m, rules[k].owner, rules[k].kind));
        ks.dedup();
        ks.truncate(3);
        let tie = (0..n)
            .filter(|&z| z != m && !fixed[m] && preference(&entries[m], &entries[z]) != 0)
            .filter(|&z| (new_at[m] > new_at[z]) != (alt_at[m] > alt_at[z]))
            .filter(|&z| (new_at[m] > new_at[z]) == (preference(&entries[m], &entries[z]) > 0))
            .max_by_key(|&z| (preference(&entries[m], &entries[z]).abs(), std::cmp::Reverse(z)))
            .map(|z| Tie {
                other: entries[z].id.clone(),
                later: new_at[m] > new_at[z],
                files: entries[m].files.iter().filter(|(k, _)| entries[z].files.contains_key(*k)).map(|(_, v)| v.clone()).collect(),
            });
        moves.push(Move { id: entries[m].id.clone(), from: m, to: new_at[m], rules: ks.iter().map(|&k| rule_out(&rules[k], entries)).collect(), tie });
    }
    moves.sort_by_key(|m| m.to);
    SortPlan { order: order_with.iter().map(|&i| entries[i].id.clone()).collect(), moves, conflicts, notes }
}

/// The keys of the files a mod's chosen resources replace, with names. The game's unit lists merge
/// instead of being replaced, and a sound bank's clips go one by one, so those count apart.
fn files_of(m: &Manifest, profile: &Profile) -> BTreeMap<String, String> {
    let values = crate::options::effective_values(&m.option_specs().unwrap_or_default(), profile.options.get(&m.id));
    let mut out = BTreeMap::new();
    for r in m.resources.iter().filter(|r| crate::options::when_matches(&r.when, &values)) {
        if crate::compose::merged_table(r.type_hash, r.name_hash).is_some() {
            continue;
        }
        let name = crate::hash::display_text(&r.name_text);
        match &r.payload {
            Payload::Clips(clips) => {
                for c in clips {
                    out.insert(format!("{:016x}:{:016x}:{:016x}", r.type_hash, r.name_hash, c.id()), name.clone());
                }
            }
            _ => {
                out.insert(format!("{:016x}:{:016x}", r.type_hash, r.name_hash), name);
            }
        }
    }
    out
}

/// The sorter's view of a profile: every mod, on or off, in the profile order.
pub fn entries(lib: &Library, profile: &Profile) -> Result<Vec<Entry>> {
    let manifests: BTreeMap<String, Manifest> = lib.list()?.into_iter().map(|m| (m.id.clone(), m)).collect();
    let locked = |f: &Option<String>| f.as_ref().is_some_and(|f| profile.folders.iter().any(|x| &x.id == f && x.locked));
    Ok(profile
        .mods
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let in_lock = locked(&e.folder);
            let sealed_after = in_lock && profile.mods.get(i + 1).is_some_and(|n| n.folder == e.folder);
            match manifests.get(&e.id).cloned().or_else(|| load_manifest(&lib.mod_dir(&e.id)).ok()) {
                Some(m) => Entry { id: e.id.clone(), rules: m.load_rules(), patch: !m.resources.is_empty(), files: files_of(&m, profile), fixed: in_lock, sealed_after },
                None => Entry { id: e.id.clone(), fixed: true, sealed_after, ..Default::default() },
            }
        })
        .collect())
}

pub fn plan(lib: &Library, profile: &Profile) -> Result<SortPlan> {
    Ok(sort(&entries(lib, profile)?))
}

impl Profile {
    /// Puts the mods in `order`, as Sort Mods worked it out. A moved mod that lands inside a folder joins
    /// it, and one that leaves the folder it was in goes out of it, so every folder stays together and
    /// the order stays as given. A locked folder takes no mod in.
    pub fn apply_sort(&mut self, order: &[String], moved: &[String]) {
        self.set_order(order);
        let is_moved = |id: &str| moved.iter().any(|m| m == id);
        let locked: Vec<String> = self.folders.iter().filter(|f| f.locked).map(|f| f.id.clone()).collect();
        let n = self.mods.len();
        // Joins: the nearest unmoved mods on both sides are in one unlocked folder.
        let still: Vec<usize> = (0..n).filter(|&i| !is_moved(&self.mods[i].id)).collect();
        let moving: Vec<usize> = (0..n).filter(|&i| is_moved(&self.mods[i].id)).collect();
        for i in moving {
            let left = still.iter().rev().find(|&&s| s < i).map(|&s| self.mods[s].folder.clone());
            let right = still.iter().find(|&&s| s > i).map(|&s| self.mods[s].folder.clone());
            if let (Some(Some(l)), Some(Some(r))) = (&left, &right)
                && l == r
                && !locked.contains(l)
            {
                self.mods[i].folder = Some(l.clone());
            }
        }
        // Leaves: each folder keeps the run with most unmoved mods, and moved mods outside it leave.
        let folders: BTreeSet<String> = self.mods.iter().filter_map(|e| e.folder.clone()).collect();
        for f in folders {
            let mut runs: Vec<Vec<usize>> = Vec::new();
            for i in 0..n {
                if self.mods[i].folder.as_deref() != Some(f.as_str()) {
                    continue;
                }
                match runs.last_mut() {
                    Some(run) if *run.last().unwrap() + 1 == i => run.push(i),
                    _ => runs.push(vec![i]),
                }
            }
            if runs.len() < 2 {
                continue;
            }
            let weight = |run: &Vec<usize>| (run.iter().filter(|&&i| !is_moved(&self.mods[i].id)).count(), run.len());
            let home = (0..runs.len()).max_by_key(|&r| (weight(&runs[r]), std::cmp::Reverse(r))).unwrap();
            for (r, run) in runs.iter().enumerate() {
                if r != home {
                    for &i in run {
                        self.mods[i].folder = None;
                    }
                }
            }
        }
        self.tidy_folders();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::ProfileEntry;

    fn e(id: &str) -> Entry {
        Entry { id: id.to_string(), ..Default::default() }
    }
    fn last(id: &str) -> Entry {
        Entry { rules: LoadRules { position: Some(Position::Last), ..Default::default() }, ..e(id) }
    }
    fn first(id: &str) -> Entry {
        Entry { rules: LoadRules { position: Some(Position::First), ..Default::default() }, ..e(id) }
    }
    fn after(id: &str, others: &[&str]) -> Entry {
        Entry { rules: LoadRules { after: others.iter().map(|s| s.to_string()).collect(), ..Default::default() }, ..e(id) }
    }
    fn before(id: &str, others: &[&str]) -> Entry {
        Entry { rules: LoadRules { before: others.iter().map(|s| s.to_string()).collect(), ..Default::default() }, ..e(id) }
    }
    fn with_files(mut x: Entry, files: &[&str]) -> Entry {
        x.patch = true;
        x.files = files.iter().map(|f| (f.to_string(), format!("lua/{f}"))).collect();
        x
    }
    fn ids(p: &SortPlan) -> Vec<&str> {
        p.order.iter().map(String::as_str).collect()
    }
    fn moved(p: &SortPlan) -> Vec<&str> {
        p.moves.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn mods_that_ask_nothing_keep_their_order() {
        let p = sort(&[e("a"), e("b"), e("c")]);
        assert_eq!(ids(&p), ["a", "b", "c"]);
        assert!(p.moves.is_empty() && p.conflicts.is_empty() && p.notes.is_empty());
        assert!(sort(&[]).order.is_empty());
    }

    #[test]
    fn last_and_first_move_only_that_mod() {
        let p = sort(&[e("a"), last("z"), e("b"), e("c")]);
        assert_eq!(ids(&p), ["a", "b", "c", "z"]);
        assert_eq!(p.moves, [Move { id: "z".into(), from: 1, to: 3, rules: vec![Rule { owner: "z".into(), kind: RuleKind::Last, other: None }], tie: None }]);
        let p = sort(&[e("a"), e("b"), first("f"), e("c")]);
        assert_eq!(ids(&p), ["f", "a", "b", "c"]);
        assert_eq!(moved(&p), ["f"]);
        // Already in place: nothing to do.
        assert!(sort(&[first("f"), e("a"), last("z")]).moves.is_empty());
    }

    #[test]
    fn before_and_after_move_the_fewest_mods() {
        let p = sort(&[after("a", &["d"]), e("b"), e("c"), e("d")]);
        assert_eq!(ids(&p), ["b", "c", "d", "a"]);
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].rules, [Rule { owner: "a".into(), kind: RuleKind::After, other: Some("d".into()) }]);
        let p = sort(&[e("a"), e("b"), before("c", &["a"])]);
        assert_eq!(ids(&p), ["c", "a", "b"]);
        // A chain: c after b, b after a, all the wrong way round.
        let p = sort(&[after("c", &["b"]), after("b", &["a"]), e("a")]);
        assert_eq!(ids(&p), ["a", "b", "c"]);
        assert_eq!(p.moves.len(), 2);
    }

    #[test]
    fn a_mod_moved_for_another_names_the_request() {
        // x must follow y; moving either is one move, and the mod that asked moves.
        let p = sort(&[e("y0"), after("x", &["y"]), e("y")]);
        assert_eq!((ids(&p), moved(&p)), (vec!["y0", "y", "x"], vec!["x"]));
        assert_eq!(p.moves[0].rules, [Rule { owner: "x".into(), kind: RuleKind::After, other: Some("y".into()) }]);
        assert_eq!((p.moves[0].from, p.moves[0].to), (1, 2));
    }

    #[test]
    fn dilworth_matches_brute_force_on_random_orders() {
        let mut seed: u64 = 0x2545f4914f6cdd1d;
        let mut rand = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for _ in 0..300 {
            let n = 10;
            let entries: Vec<Entry> = (0..n).map(|i| e(&format!("m{i}"))).collect();
            let rules: Vec<RawRule> = (0..rand(12)).filter_map(|_| {
                let (a, b) = (rand(n) as usize, rand(n) as usize);
                (a < b).then_some(RawRule { owner: a, kind: RuleKind::After, other: Some(b) })
            }).collect();
            let g = graph(&entries, &rules, &vec![false; n as usize]);
            let inv = |a: usize, b: usize| if a < b { g.reach[b].get(a) } else { g.reach[a].get(b) };
            let pool: Vec<usize> = (0..n as usize).collect();
            let brute = keep_sets(&pool, &inv)[0].len();
            let d = dilworth(&pool, &inv);
            assert_eq!(d.len(), brute);
            assert!(d.iter().all(|&a| d.iter().all(|&b| a == b || !inv(a, b))), "an antichain");
        }
    }

    #[test]
    fn patches_rules_go_around_every_mod_with_files() {
        let p = sort(&[with_files(e("p1"), &["f1"]), e("plain"), Entry { rules: LoadRules { before_patches: true, ..Default::default() }, ..e("early") }, with_files(e("p2"), &["f2"])]);
        assert_eq!(ids(&p), ["early", "p1", "plain", "p2"]);
        let p = sort(&[with_files(e("p1"), &["f1"]), Entry { rules: LoadRules { after_patches: true, ..Default::default() }, ..e("late") }, e("plain"), with_files(e("p2"), &["f2"])]);
        assert_eq!(ids(&p), ["p1", "plain", "p2", "late"]);
        assert_eq!(p.moves[0].rules[0].kind, RuleKind::AfterPatches);
    }

    #[test]
    fn two_mods_asking_for_the_same_end_stay_and_are_reported() {
        let p = sort(&[last("a"), e("b"), last("c"), e("d")]);
        assert_eq!(ids(&p), ["a", "b", "c", "d"]);
        assert!(p.moves.is_empty());
        assert_eq!(p.conflicts.len(), 1);
        assert_eq!((p.conflicts[0].kind, p.conflicts[0].mods.clone()), (ClashKind::BothLast, vec!["a".to_string(), "c".to_string()]));
        let p = sort(&[e("a"), first("b"), first("c")]);
        assert_eq!(p.conflicts[0].kind, ClashKind::BothFirst);
        assert_eq!(ids(&p), ["a", "b", "c"]);
    }

    #[test]
    fn a_cycle_or_a_rule_against_a_position_is_reported_and_the_rest_still_sorts() {
        let p = sort(&[after("a", &["b"]), after("b", &["a"]), last("z"), e("c")]);
        assert_eq!(p.conflicts.len(), 1);
        assert_eq!((p.conflicts[0].kind, p.conflicts[0].mods.clone()), (ClashKind::Cycle, vec!["a".to_string(), "b".to_string()]));
        assert_eq!(p.conflicts[0].rules.len(), 2);
        assert_eq!(ids(&p), ["a", "b", "c", "z"], "the clash stays, the last mod still goes last");
        // A mod asking to come after the one that asks to be last.
        let p = sort(&[last("z"), after("a", &["z"]), e("b")]);
        assert_eq!(p.conflicts[0].kind, ClashKind::Cycle);
        assert_eq!(p.conflicts[0].mods, ["z", "a"]);
        assert!(p.moves.is_empty());
        let kinds: Vec<RuleKind> = p.conflicts[0].rules.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Last) && kinds.contains(&RuleKind::After), "{kinds:?}");
    }

    #[test]
    fn a_name_not_in_the_profile_is_a_note() {
        let p = sort(&[after("a", &["gone.mod"]), e("b")]);
        assert_eq!(p.notes, [Note { owner: "a".into(), missing: "gone.mod".into() }]);
        assert!(p.moves.is_empty());
    }

    #[test]
    fn the_mod_with_fewer_files_goes_later_where_the_moves_tie() {
        // `small` asks to come after `anchor`; it can sit right after `anchor` or after `big` too.
        // Going after `big` it overwrites 1 of the 3 files of `big`; the other way, `big` would wipe out
        // the one file of `small`.
        let entries = [with_files(after("small", &["anchor"]), &["s"]), e("anchor"), with_files(e("big"), &["s", "b1", "b2"])];
        let p = sort(&entries);
        assert_eq!(ids(&p), ["anchor", "big", "small"]);
        let tie = p.moves[0].tie.clone().expect("the file rule decided");
        assert_eq!((tie.other.as_str(), tie.later, tie.files.clone()), ("big", true, vec!["lua/s".to_string()]));
        // Without shared files, the nearest place.
        let p = sort(&[after("small", &["anchor"]), e("anchor"), with_files(e("big"), &["x"])]);
        assert_eq!(ids(&p), ["anchor", "small", "big"]);
        assert!(p.moves[0].tie.is_none());
    }

    #[test]
    fn the_file_rule_chooses_which_mod_moves() {
        // `a` asks to come after `b`: moving `a` to the end or `b` to the top is one move either way.
        // With nothing else to go by, the mod that asked moves.
        let p = sort(&[after("a", &["b"]), e("z"), e("b")]);
        assert_eq!((ids(&p), moved(&p)), (vec!["z", "b", "a"], vec!["a"]));
        // `z` shares a file with `a` and has fewer files, so `z` should stay after `a`: `b` moves instead.
        let p = sort(&[with_files(after("a", &["b"]), &["s", "t", "u"]), with_files(e("z"), &["s"]), e("b")]);
        assert_eq!((ids(&p), moved(&p)), (vec!["b", "a", "z"], vec!["b"]));
        assert_eq!(p.moves[0].rules, [Rule { owner: "a".into(), kind: RuleKind::After, other: Some("b".into()) }]);
    }

    #[test]
    fn locked_mods_stay_and_nothing_goes_inside_their_folder() {
        let mut l1 = e("l1");
        l1.fixed = true;
        l1.sealed_after = true;
        let mut l2 = e("l2");
        l2.fixed = true;
        // `a` asks to load last but `l2` (locked) is last: `a` goes after it anyway, as it is outside the folder.
        let p = sort(&[e("x"), last("a"), l1.clone(), l2.clone()]);
        assert_eq!(ids(&p), ["x", "l1", "l2", "a"]);
        // `a` must sit between two mods of a locked folder: it can't, so it stays and is reported.
        let a = Entry { rules: LoadRules { after: vec!["l1".into()], before: vec!["l2".into()], ..Default::default() }, ..e("a") };
        let p = sort(&[a, l1.clone(), l2.clone()]);
        assert_eq!(ids(&p), ["a", "l1", "l2"]);
        assert_eq!((p.conflicts[0].kind, p.conflicts[0].mods.clone()), (ClashKind::Locked, vec!["a".to_string()]));
        assert_eq!(p.conflicts[0].rules.len(), 2);
        // A locked mod asking to come after another locked mod above it: the request can't move either.
        let mut l3 = after("l3", &["l4"]);
        l3.fixed = true;
        let mut l4 = e("l4");
        l4.fixed = true;
        let p = sort(&[l3, e("y"), l4]);
        assert_eq!(ids(&p), ["l3", "y", "l4"]);
        assert_eq!(p.conflicts[0].kind, ClashKind::Locked);
    }

    #[test]
    fn many_mods_out_of_place_still_take_the_fewest_moves() {
        // 20 mods, each asking to come after the next one: the fewest moves turn the list around but one.
        let n = 20;
        let entries: Vec<Entry> = (0..n).map(|i| if i + 1 < n { after(&format!("m{i:02}"), &[&format!("m{:02}", i + 1)]) } else { e(&format!("m{i:02}")) }).collect();
        let p = sort(&entries);
        let want: Vec<String> = (0..n).rev().map(|i| format!("m{i:02}")).collect();
        assert_eq!(p.order, want);
        assert_eq!(p.moves.len(), n - 1);
        // And a long list with one mod out of place moves one.
        let mut many: Vec<Entry> = (0..40).map(|i| e(&format!("k{i:02}"))).collect();
        many[3] = last("k03");
        many.push(after("late", &["k39"]));
        many.swap(0, 40);
        let p = sort(&many);
        assert_eq!(p.moves.len(), 2);
        assert_eq!(p.order.last().unwrap(), "k03");
    }

    #[test]
    fn dilworth_finds_as_many_kept_mods_as_brute_force() {
        // 18 out-of-place mods in pairs: one of each pair moves.
        let mut entries = Vec::new();
        for i in 0..9 {
            entries.push(after(&format!("a{i}"), &[&format!("b{i}")]));
            entries.push(e(&format!("b{i}")));
        }
        let p = sort(&entries);
        assert_eq!(p.moves.len(), 9);
        for i in 0..9 {
            let at = |id: String| p.order.iter().position(|o| *o == id).unwrap();
            assert!(at(format!("b{i}")) < at(format!("a{i}")));
        }
    }

    /// Random small lists against every order there is: when nothing clashes, every request holds and
    /// no order that keeps them all moves fewer mods.
    #[test]
    fn random_lists_hold_every_request_with_the_fewest_moves() {
        let mut seed: u64 = 0x9e3779b97f4a7c15;
        let mut rand = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        fn perms(n: usize) -> Vec<Vec<usize>> {
            if n == 0 {
                return vec![vec![]];
            }
            perms(n - 1).into_iter().flat_map(|p| (0..n).map(move |at| { let mut q = p.clone(); q.insert(at, n - 1); q })).collect()
        }
        let all = perms(6);
        let lis = |p: &[usize]| {
            let mut tails: Vec<usize> = Vec::new();
            for &x in p {
                let at = tails.partition_point(|&t| t < x);
                if at == tails.len() { tails.push(x) } else { tails[at] = x }
            }
            tails.len()
        };
        let mut checked = 0;
        for _ in 0..400 {
            let n = 6;
            let mut entries: Vec<Entry> = (0..n).map(|i| e(&format!("m{i}"))).collect();
            for x in entries.iter_mut() {
                match rand(9) {
                    0 => x.rules.position = Some(Position::Last),
                    1 => x.rules.position = Some(Position::First),
                    2 | 3 => x.rules.after.push(format!("m{}", rand(n as u64))),
                    4 => x.rules.before.push(format!("m{}", rand(n as u64))),
                    5 => x.rules.after_patches = true,
                    _ => {}
                }
                if rand(3) == 0 {
                    x.patch = true;
                    x.files.insert(format!("f{}", rand(3)), "f".into());
                }
            }
            let p = sort(&entries);
            if !p.conflicts.is_empty() {
                continue;
            }
            checked += 1;
            let (rules, _) = collect_rules(&entries);
            let holds = |order: &[usize]| {
                let mut at = vec![0; n];
                for (k, &x) in order.iter().enumerate() {
                    at[x] = k;
                }
                rules.iter().all(|r| edges_of(r, &entries).iter().all(|&(u, v)| at[u] < at[v]))
            };
            let got: Vec<usize> = p.order.iter().map(|id| entries.iter().position(|x| &x.id == id).unwrap()).collect();
            assert!(holds(&got), "{:?} -> {:?}", entries.iter().map(|x| &x.rules).collect::<Vec<_>>(), p.order);
            let fewest = all.iter().filter(|o| holds(o)).map(|o| n - lis(o)).min().unwrap();
            assert_eq!(p.moves.len(), fewest, "{:?} -> {:?}", entries.iter().map(|x| &x.rules).collect::<Vec<_>>(), p.order);
        }
        assert!(checked > 100, "only {checked} lists without a clash");
    }

    fn profile(ids: &[(&str, Option<&str>)]) -> Profile {
        let mut p = Profile { mods: ids.iter().map(|(id, f)| ProfileEntry { id: id.to_string(), enabled: true, folder: f.map(str::to_string) }).collect(), ..Default::default() };
        for f in ["f", "g"] {
            p.folders.push(crate::folders::Folder { id: f.into(), name: f.into(), color: "#fff".into(), collapsed: false, locked: false });
        }
        p
    }
    fn order_of(p: &Profile) -> Vec<(&str, Option<&str>)> {
        p.mods.iter().map(|m| (m.id.as_str(), m.folder.as_deref())).collect()
    }
    fn s(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn applying_keeps_folders_together_and_the_order_as_given() {
        // `x` leaves folder f for the end; `y` lands between two mods of g and joins it.
        let mut p = profile(&[("x", Some("f")), ("f1", Some("f")), ("g1", Some("g")), ("g2", Some("g")), ("y", None), ("z", None)]);
        p.apply_sort(&s(&["f1", "g1", "y", "g2", "z", "x"]), &s(&["x", "y"]));
        assert_eq!(order_of(&p), [("f1", Some("f")), ("g1", Some("g")), ("y", Some("g")), ("g2", Some("g")), ("z", None), ("x", None)]);
        // A moved mod that stays beside its folder keeps it.
        let mut p = profile(&[("f1", Some("f")), ("x", Some("f")), ("f2", Some("f")), ("z", None)]);
        p.apply_sort(&s(&["x", "f1", "f2", "z"]), &s(&["x"]));
        assert_eq!(order_of(&p), [("x", Some("f")), ("f1", Some("f")), ("f2", Some("f")), ("z", None)]);
        // A locked folder takes nothing in.
        let mut p = profile(&[("g1", Some("g")), ("g2", Some("g")), ("y", None)]);
        p.folders[1].locked = true;
        p.apply_sort(&s(&["g1", "y", "g2"]), &s(&["y"]));
        assert_eq!(p.mods.iter().find(|m| m.id == "y").unwrap().folder, None);
    }

    #[test]
    fn entries_come_from_the_library_with_files_and_locks() {
        use crate::testutil::{put, spaced_tempdir};
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let stage = |id: &str, extra: &str| {
            let d = t.path().join(format!("stage-{id}"));
            put(&d, "obsidian.json", format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}}{extra}}}"#).as_bytes());
            put(&d, "a.lua", b"x");
            lib.install(&d).unwrap();
        };
        stage("plain", "");
        stage("files", r#","resources":[{"type":"lua","name":"lua/settings/perk_settings","source":"a.lua"},{"type":"lua","name":"lua/generated/unit_paths","source":"a.lua"}]"#);
        stage("late", r#","load":"last""#);
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.set_order(&s(&["late", "files", "plain"]));
        p.folders.push(crate::folders::Folder { id: "f".into(), name: "F".into(), color: "#fff".into(), collapsed: false, locked: true });
        p.mods[1].folder = Some("f".into());
        p.mods[2].folder = Some("f".into());
        let list = entries(&lib, &p).unwrap();
        assert_eq!(list[0].rules.position, Some(Position::Last));
        assert!(list[1].patch && list[1].fixed && list[1].sealed_after);
        assert_eq!(list[1].files.values().collect::<Vec<_>>(), ["lua/settings/perk_settings"], "a merged table isn't a file that one mod wipes out");
        assert!(list[2].fixed && !list[2].sealed_after);
        let plan = plan(&lib, &p).unwrap();
        assert_eq!(plan.order, ["files", "plain", "late"]);
        p.apply_sort(&plan.order, &plan.moves.iter().map(|m| m.id.clone()).collect::<Vec<_>>());
        assert_eq!(order_of(&p), [("files", Some("f")), ("plain", Some("f")), ("late", None)]);
    }
}
