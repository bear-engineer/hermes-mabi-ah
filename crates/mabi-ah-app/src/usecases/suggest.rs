//! "Did you mean" for option, enchant and reforge names.
//!
//! Names are compared on Hangul jamo (each syllable split into initial, medial and
//! final) with whitespace removed, so "세공옵션" matches "세공 옵션" and a single
//! wrong final consonant ("날카론운") costs one edit. A candidate that contains the
//! input (or is contained in it) ranks before edit-distance matches.

/// At most this many suggestions are returned.
pub const MAX_SUGGESTIONS: usize = 5;

const S_BASE: u32 = 0xAC00;
const S_LAST: u32 = 0xD7A3;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;

/// Whitespace-free, lowercase jamo sequence of `s`.
fn jamo(s: &str) -> Vec<char> {
    let mut out = Vec::new();
    for c in s.chars().filter(|c| !c.is_whitespace()) {
        let code = u32::from(c);
        if (S_BASE..=S_LAST).contains(&code) {
            let i = code - S_BASE;
            let (l, v, t) = (
                i / (V_COUNT * T_COUNT),
                (i / T_COUNT) % V_COUNT,
                i % T_COUNT,
            );
            out.extend(char::from_u32(L_BASE + l));
            out.extend(char::from_u32(V_BASE + v));
            if t != 0 {
                out.extend(char::from_u32(T_BASE + t));
            }
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn contains(hay: &[char], needle: &[char]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// Up to [`MAX_SUGGESTIONS`] distinct candidates close to `input`: substring
/// matches first, then by jamo edit distance (at most a third of the input's jamo,
/// minimum 2), ties by candidate text.
pub fn suggest_option_names(input: &str, candidates: &[String]) -> Vec<String> {
    let want = jamo(input);
    if want.is_empty() {
        return Vec::new();
    }
    let limit = (want.len() / 3).max(2);
    let mut scored: Vec<(bool, usize, &str)> = Vec::new();
    for c in candidates {
        if scored.iter().any(|(_, _, s)| *s == c.as_str()) {
            continue;
        }
        let have = jamo(c);
        let substring = contains(&have, &want) || contains(&want, &have);
        let dist = edit_distance(&want, &have);
        if substring || dist <= limit {
            scored.push((!substring, dist, c.as_str()));
        }
    }
    scored.sort_unstable();
    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, s)| s.to_owned())
        .collect()
}
