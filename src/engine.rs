// Bijoy52 longest-match engine — direct Rust port of the same state machine
// used in the Obsidian plugin (`main.js`) and the AutoHotkey script, so all
// three versions stay behaviorally identical. See those files' comments for
// the origin of the rule set (bn-bijoyUnicode.mim, Ananda Computers).

use std::collections::{HashMap, HashSet, VecDeque};

pub struct BijoyEngine {
    map: HashMap<String, String>,
    extendable: HashSet<String>,
    shorborno: HashMap<char, String>,
    buf: String,
    last_len: usize,
    last_output: String,
    last_keyseq: String,
}

impl BijoyEngine {
    pub fn new() -> Self {
        let raw = include_str!("../rules.json");
        let map: HashMap<String, String> =
            serde_json::from_str(raw).expect("rules.json must be valid JSON");

        let mut extendable = HashSet::new();
        for key in map.keys() {
            let chars: Vec<char> = key.chars().collect();
            for i in 1..chars.len() {
                extendable.insert(chars[..i].iter().collect());
            }
        }

        let shorborno: HashMap<char, String> = [
            ('f', "আ"),
            ('d', "ই"),
            ('D', "ঈ"),
            ('s', "উ"),
            ('S', "ঊ"),
            ('a', "ঋ"),
            ('c', "এ"),
            ('C', "ঐ"),
            ('x', "ও"),
            ('X', "ঔ"),
            ('G', "॥"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();

        Self {
            map,
            extendable,
            shorborno,
            buf: String::new(),
            last_len: 0,
            last_output: String::new(),
            last_keyseq: String::new(),
        }
    }

    pub fn reset_partial(&mut self) {
        self.buf.clear();
        self.last_len = 0;
        self.last_output.clear();
        self.last_keyseq.clear();
    }

    /// Feed one raw keystroke in. Returns the text (possibly empty) that
    /// should be injected in its place.
    pub fn feed(&mut self, raw_key: char) -> String {
        let mut append = String::new();
        let mut queue: VecDeque<char> = VecDeque::new();
        queue.push_back(raw_key);

        while let Some(k) = queue.pop_front() {
            let mut new_buf = self.buf.clone();
            new_buf.push(k);

            let has_exact = self.map.contains_key(&new_buf);
            let is_extendable = self.extendable.contains(&new_buf);

            if has_exact || is_extendable {
                self.buf = new_buf.clone();
                if has_exact {
                    self.last_len = new_buf.chars().count();
                    self.last_output = self.map.get(&new_buf).unwrap().clone();
                    self.last_keyseq = new_buf.clone();
                }
                if !is_extendable {
                    append.push_str(&self.commit());
                }
            } else if self.last_len > 0 {
                let output = self.last_output.clone();
                let keyseq = self.last_keyseq.clone();
                let buf_chars: Vec<char> = self.buf.chars().collect();
                let leftover: String = buf_chars[self.last_len..].iter().collect();
                self.reset_partial();

                if keyseq == "g" && leftover.is_empty() {
                    if let Some(v) = self.shorborno.get(&k) {
                        append.push_str(v);
                        continue;
                    }
                }
                append.push_str(&output);
                for c in leftover.chars() {
                    queue.push_back(c);
                }
                queue.push_back(k);
            } else if !self.buf.is_empty() {
                let buf_chars: Vec<char> = self.buf.chars().collect();
                let literal = buf_chars[0];
                append.push(literal);
                let rest: String = buf_chars[1..].iter().collect();
                self.reset_partial();
                for c in rest.chars() {
                    queue.push_back(c);
                }
                queue.push_back(k);
            } else {
                append.push(k); // unmapped key: literal passthrough
            }
        }

        append
    }

    fn commit(&mut self) -> String {
        let output = self.last_output.clone();
        self.reset_partial();
        output
    }

    /// Commit whatever is pending without a new keystroke arriving (e.g. a
    /// deliberate pause, or a word boundary like Space). Mirrors the JS
    /// engine's `flushPending()`, used by tests to finalize a word the same
    /// way real typing would via the next keystroke or an idle pause.
    /// Not wired into `main.rs` yet (see README's "Known limitations" —
    /// no auto-flush-on-pause timer in v1); kept public for that fast-follow.
    #[allow(dead_code)]
    pub fn flush_pending(&mut self) -> String {
        let result = if self.last_len > 0 {
            self.last_output.clone()
        } else if let Some(c) = self.buf.chars().next() {
            c.to_string()
        } else {
            String::new()
        };
        self.reset_partial();
        result
    }

    /// Undo the last keystroke fed into the pending buffer. Returns true if
    /// there was something to undo (mid-sequence undo — nothing was shown on
    /// screen yet, so the caller must suppress the physical Backspace too).
    pub fn backspace(&mut self) -> bool {
        if !self.buf.is_empty() {
            let mut keys: Vec<char> = self.buf.chars().collect();
            keys.pop();
            self.reset_partial();
            for kk in keys {
                self.feed(kk);
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    // Feeds a full key sequence and then flushes, mirroring how a real word
    // ends: either the next differing keystroke or a pause commits whatever
    // is still buffered. Without this, sequences that are also a prefix of
    // some longer rule (very common in a ~3,880-rule set) would legitimately
    // sit unflushed, since the engine can't know no more keys are coming.
    fn feed_str(engine: &mut BijoyEngine, s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            out.push_str(&engine.feed(c));
        }
        out.push_str(&engine.flush_pending());
        out
    }

    #[test]
    fn every_rule_round_trips_in_isolation() {
        let raw = include_str!("../rules.json");
        let rules: StdHashMap<String, String> = serde_json::from_str(raw).unwrap();
        let mut engine = BijoyEngine::new();
        let mut failures = vec![];
        for (keyseq, expected) in &rules {
            engine.reset_partial();
            let got = feed_str(&mut engine, keyseq);
            if &got != expected {
                failures.push(format!("{keyseq:?}: expected {expected:?}, got {got:?}"));
            }
        }
        assert!(
            failures.is_empty(),
            "{} / {} rules failed:\n{}",
            failures.len(),
            rules.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn known_words() {
        let mut engine = BijoyEngine::new();
        assert_eq!(feed_str(&mut engine, "gfdm"), "আমি");
        let mut engine = BijoyEngine::new();
        assert_eq!(feed_str(&mut engine, "ufbd"), "জানি");
        let mut engine = BijoyEngine::new();
        assert_eq!(feed_str(&mut engine, "dj"), "কি");
        let mut engine = BijoyEngine::new();
        assert_eq!(feed_str(&mut engine, "gf"), "আ");
    }

    #[test]
    fn backspace_mid_sequence_shows_nothing() {
        let mut engine = BijoyEngine::new();
        let _ = engine.feed('g');
        assert!(engine.backspace());
    }
}
