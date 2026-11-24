//! Round 2 of the security review of
//! story:standing-grant-binds-a-pattern-or-a-value-set, after the fix round
//! that rebuilt the pattern matcher from `regex-automata` and refused a
//! pattern that folds case while Unicode matching is on. Every test drives
//! the public reader `local::standing::parse` and the check of
//! `approvals grant-standing`, `local::standing::check_request`.
//!
//! - No pattern the reader takes reaches the Unicode case fold, and the
//!   reader refuses exactly the patterns the texts say it refuses.
//! - `grant-standing` and the reader refuse the same patterns.
//! - The matcher answers as `regex::Regex` does on `\A(?:pattern)\z`.
//! - The other measured figures of contracts/service/compatibility.md § 2.1.
//! - What the check of one write costs against a file within the stated
//!   limits.
#![cfg(unix)]
use crate::support::LIVE;
use connectors_host::local::standing::{self, Grants, Refused, Unreadable};
use serde_json::{Value, json};
use std::{cell::Cell, time::Duration};

const INSTANCE: &str = "leaf";
const OPERATION: &str = "issue.transition.run";
const LATER: i64 = 2_000_000_000_000;
/// U+212A KELVIN SIGN, which the Unicode case fold gives `k`.
const KELVIN: char = '\u{212a}';
/// U+017F LATIN SMALL LETTER LONG S, which the Unicode case fold gives `s`.
const LONG_S: char = '\u{17f}';

fn live() -> isize {
    LIVE.with(Cell::get)
}

/// The CPU time this thread has used.
fn cpu() -> Duration {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid timespec this call writes.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    assert_eq!(status, 0);
    Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

/// A grant file of `members` grants, each with the one target member
/// `{"pointer": "/k", "pattern": pattern}`.
fn file_of(pattern: &str, members: usize) -> Vec<u8> {
    let grants: Vec<Value> = (0..members)
        .map(|n| {
            json!({
                "id": format!("g{n}"), "instance": INSTANCE, "operation": OPERATION,
                "target": [{"pointer": "/k", "pattern": pattern}],
                "not_after_unix_ms": LATER, "max_per_hour": 1
            })
        })
        .collect();
    serde_json::to_vec(&json!({"format": standing::FORMAT, "grants": grants, "revoked": []}))
        .unwrap()
}

/// The standing reader's answer for a file whose one member holds `pattern`.
fn read(pattern: &str) -> Result<Grants, Unreadable> {
    standing::parse(&file_of(pattern, 1))
}

/// The answer of `approvals grant-standing`'s check for a request whose one
/// member holds `pattern`.
fn requested(pattern: &str) -> Result<(), Refused> {
    let request = json!({
        "operation": OPERATION,
        "target": [{"pointer": "/k", "pattern": pattern}],
        "not_after_unix_ms": LATER, "max_per_hour": 1
    });
    standing::check_request(&serde_json::to_vec(&request).unwrap(), INSTANCE)
}

/// Whether the grant of `grants` covers the write `{"k": value}`.
fn admits(grants: &Grants, value: &str) -> bool {
    grants
        .covering(INSTANCE, OPERATION, &json!({"k": value}), 1_000)
        .is_ok_and(|covering| covering.len() == 1)
}

/// `\A(?:pattern)\z` as the `regex` crate reads it with all its engines,
/// under limits no pattern here meets.
fn reference(pattern: &str) -> Option<regex::Regex> {
    regex::RegexBuilder::new(&format!(r"\A(?:{pattern})\z"))
        .size_limit(1 << 28)
        .dfa_size_limit(1 << 28)
        .build()
        .ok()
}

/// A fixed-seed generator (xorshift64).
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len())]
    }
}

/// The flags of the place a generated pattern is written at.
#[derive(Clone, Copy)]
struct Flags {
    fold: bool,
    unicode: bool,
    verbose: bool,
}

/// Writes a pattern piece by piece and keeps what the syntax of the `regex`
/// crate says about it: `(?flags)` holds to the end of the group it stands
/// in, over a `|` too; `(?flags:...)` holds inside its group; under `x` a `#`
/// starts a comment that ends with the line.
struct Writer {
    random: Random,
    text: String,
    flags: Flags,
    /// A flag was set that left case-insensitive matching and Unicode
    /// matching both on: the rule the reader refuses a pattern by.
    folds_unicode: bool,
    groups: usize,
}

/// What an atom is written as, and values it can match: as written, with an
/// ASCII fold, with the Unicode fold. No atom matches U+212A or U+017F as
/// written.
const ATOMS: &[(&str, &[&str])] = &[
    ("k", &["k", "K", "\u{212a}"]),
    ("s", &["s", "S", "\u{17f}"]),
    ("[k]", &["k", "K", "\u{212a}"]),
    ("[j-l]", &["k", "K", "\u{212a}"]),
    (r"\x6B", &["k", "K", "\u{212a}"]),
    ("[a-z]", &["k", "S", "\u{212a}", "\u{17f}"]),
    ("[[:lower:]]", &["s", "K", "\u{17f}"]),
    ("[(?i)]", &["i", "I", "?"]),
    (r"\(\?i\)", &["(?i)"]),
];

/// Flags as `(?flags)` and `(?flags:...)` take them.
const DIRECTIVES: &[&str] = &[
    "i", "i", "-i", "u", "-u", "-u", "-u", "i-u", "i-u", "i-u", "u-i", "iu", "-iu", "-ui", "s",
    "x", "x", "-x", "m", "U", "R", "si", "i-s", "ix", "x-u", "sm-i", "i-ux", "is-u", "-u-i",
];

/// A text that is a comment under `x` and holds a flag otherwise.
const COMMENTS: &[(&str, &str)] = &[
    ("#(?i)\n", "i"),
    ("#(?i-u)\n", "i-u"),
    ("#(?u)\n", "u"),
    ("#(?-x)\n", "-x"),
];

impl Writer {
    fn new(seed: u64) -> Self {
        Self {
            random: Random(seed),
            text: String::new(),
            flags: Flags {
                fold: false,
                unicode: true,
                verbose: false,
            },
            folds_unicode: false,
            groups: 0,
        }
    }

    fn set(&mut self, directive: &str) {
        let mut on = true;
        for flag in directive.chars() {
            match flag {
                '-' => on = false,
                'i' => self.flags.fold = on,
                'u' => self.flags.unicode = on,
                'x' => self.flags.verbose = on,
                _ => {}
            }
        }
        if self.flags.fold && self.flags.unicode {
            self.folds_unicode = true;
        }
    }

    /// At most 24 of `samples`.
    fn thin(&mut self, mut samples: Vec<String>) -> Vec<String> {
        samples.sort();
        samples.dedup();
        while samples.len() > 24 {
            let at = self.random.below(samples.len());
            samples.swap_remove(at);
        }
        samples
    }

    fn repeated(&mut self, mut samples: Vec<String>) -> Vec<String> {
        match self.random.below(10) {
            0 => {
                self.text.push('?');
                samples.push(String::new());
            }
            1 => {
                self.text.push('*');
                samples.push(String::new());
            }
            2 => self.text.push('+'),
            3 => {
                self.text.push_str("{2}");
                samples = samples.iter().map(|one| format!("{one}{one}")).collect();
            }
            _ => {}
        }
        samples
    }

    fn piece(&mut self, depth: usize) -> Vec<String> {
        match self.random.below(if depth < 3 { 11 } else { 7 }) {
            0..=2 => {
                let (text, matches) = self.random.pick(ATOMS);
                self.text.push_str(text);
                let samples = matches.iter().map(|&one| one.to_owned()).collect();
                self.repeated(samples)
            }
            3..=5 => {
                let directive = self.random.pick(DIRECTIVES);
                self.text.push_str(&format!("(?{directive})"));
                self.set(directive);
                vec![String::new()]
            }
            6 => {
                let (text, directive) = self.random.pick(COMMENTS);
                self.text.push_str(text);
                if self.flags.verbose {
                    vec![String::new()]
                } else {
                    self.set(directive);
                    vec!["#\n".to_owned()]
                }
            }
            7..=8 => {
                let directive = self.random.pick(DIRECTIVES);
                self.text.push_str(&format!("(?{directive}:"));
                let outer = self.flags;
                self.set(directive);
                let samples = self.alternation(depth + 1);
                self.text.push(')');
                self.flags = outer;
                self.repeated(samples)
            }
            _ => {
                self.groups += 1;
                let open = match self.random.below(4) {
                    0 => "(".to_owned(),
                    1 => "(?:".to_owned(),
                    2 => format!("(?P<g{}>", self.groups),
                    _ => format!("(?<g{}>", self.groups),
                };
                self.text.push_str(&open);
                let outer = self.flags;
                let samples = self.alternation(depth + 1);
                self.text.push(')');
                self.flags = outer;
                self.repeated(samples)
            }
        }
    }

    fn concat(&mut self, depth: usize) -> Vec<String> {
        let mut samples = vec![String::new()];
        for _ in 0..1 + self.random.below(3) {
            let next = self.piece(depth);
            let joined = samples
                .iter()
                .flat_map(|head| next.iter().map(move |tail| format!("{head}{tail}")))
                .collect();
            samples = self.thin(joined);
        }
        samples
    }

    fn alternation(&mut self, depth: usize) -> Vec<String> {
        let mut samples = self.concat(depth);
        while self.random.below(4) == 0 {
            self.text.push('|');
            let other = self.concat(depth);
            samples.extend(other);
        }
        self.thin(samples)
    }
}

/// Every string of at most two of the letters a fold tells apart.
fn short_values() -> Vec<String> {
    let letters = ['k', 'K', KELVIN, 's', LONG_S];
    let mut values = vec![String::new()];
    for first in letters {
        values.push(first.to_string());
        for second in letters {
            values.push(format!("{first}{second}"));
        }
    }
    values
}

/// Invariants 2, 3 and 4 over 3,000 generated patterns (fixed seed). The
/// atoms of a pattern are ASCII letters and ASCII classes, so a value with
/// U+212A or U+017F can match only through the Unicode case fold.
///
/// - The reader takes a pattern exactly when the `regex` crate compiles it,
///   alone and wrapped, and no flag in it leaves case-insensitive matching
///   and Unicode matching both on.
/// - `grant-standing`'s check takes exactly the patterns the reader takes.
/// - A pattern the reader takes matches no value with U+212A or U+017F.
/// - A pattern the reader takes answers every value as `regex::Regex` does
///   on `\A(?:pattern)\z`.
#[test]
fn a_generated_pattern_is_taken_by_the_stated_rule_and_never_folds_beyond_ascii() {
    let short = short_values();
    let (mut taken, mut refused_by_rule, mut refused_by_syntax) = (0, 0, 0);
    let (mut ascii_folds, mut unicode_folds, mut unfolded_refusals) = (0, 0, 0);
    let mut unfolded = Vec::new();
    for seed in 1..=3_000u64 {
        let mut writer = Writer::new(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let body_samples = writer.alternation(0);
        let pattern = format!("^{}$", writer.text);
        if pattern.len() > 256 {
            continue;
        }
        let compiles = regex::Regex::new(&pattern).is_ok() && reference(&pattern).is_some();
        let expected = compiles && !writer.folds_unicode;
        let grants = read(&pattern);
        assert_eq!(
            grants.is_ok(),
            expected,
            "{pattern:?}: the regex crate compiles it: {compiles}; a flag leaves the fold and \
             Unicode both on: {}; the reader answers {:?}",
            writer.folds_unicode,
            grants.as_ref().err()
        );
        assert_eq!(
            requested(&pattern).is_ok(),
            grants.is_ok(),
            "{pattern:?}: grant-standing answers {:?}, the reader {:?}",
            requested(&pattern),
            grants.as_ref().err()
        );
        let values: Vec<&String> = short.iter().chain(&body_samples).collect();
        let exotic = |value: &str| value.contains(KELVIN) || value.contains(LONG_S);
        match grants {
            Err(refusal) => {
                assert_eq!(refusal, Unreadable::Malformed, "{pattern:?}");
                if !compiles {
                    refused_by_syntax += 1;
                    continue;
                }
                refused_by_rule += 1;
                let whole = reference(&pattern).unwrap();
                if values
                    .iter()
                    .any(|value| exotic(value) && whole.is_match(value))
                {
                    unicode_folds += 1;
                } else {
                    unfolded_refusals += 1;
                    if unfolded.len() < 5 {
                        unfolded.push(pattern.clone());
                    }
                }
            }
            Ok(grants) => {
                taken += 1;
                let whole = reference(&pattern).unwrap();
                let mut folded = false;
                for value in values {
                    let answer = admits(&grants, value);
                    assert_eq!(
                        answer,
                        whole.is_match(value),
                        "{pattern:?} on {value:?}: the reader and regex::Regex differ"
                    );
                    assert!(
                        !(answer && exotic(value)),
                        "{pattern:?} is taken and matches {value:?}: the Unicode case fold"
                    );
                    folded |= answer && value.contains(['K', 'S']);
                }
                ascii_folds += usize::from(folded);
            }
        }
    }
    println!(
        "taken {taken} (with an ASCII fold that matched: {ascii_folds}), refused by the rule \
         {refused_by_rule} (a value with U+212A or U+017F matches under regex::Regex: \
         {unicode_folds}; none found: {unfolded_refusals}, such as {unfolded:?}), refused as \
         the regex crate refuses them {refused_by_syntax}"
    );
    // The generator reaches each side often enough to mean something.
    assert!(taken >= 300, "{taken}");
    assert!(ascii_folds >= 30, "{ascii_folds}");
    assert!(unicode_folds >= 300, "{unicode_folds}");
}

/// Invariant 2 on the forms the review brief names: a flag inside a group,
/// in nested groups, after a `|`, reset by `(?-i)`, before a class and a
/// `\p{..}`, at the end, and under `x` beside comments. Each refused pattern
/// is one the `regex` crate compiles; each taken one folds ASCII letters at
/// most.
#[test]
fn the_named_forms_of_a_case_fold_are_refused_or_fold_ascii_only() {
    let refused = [
        // Inside a group, in nested groups, in a named group.
        "^(a(?i)k)$",
        "^(a(b(c(?i)k)))$",
        "^(?:a(?:b(?i:k)))$",
        "^(?P<n>(?i)k)$",
        "^(?:(?i)k)+$",
        // After a `|`, and in the branch before one.
        "^a|(?i)k$",
        "^(?:a|(?i)k|s)$",
        "^(?:(?i)a|k)$",
        // Turned off again: `a` was folded.
        "^(?i)a(?-i)k$",
        "^(?i:a)(?-i:k)$",
        // Classes and Unicode classes.
        "^(?i)[k]$",
        "^(?i:[j-l])$",
        r"^(?i)\p{Lu}$",
        r"^(?i:\p{Greek})$",
        "^(?i)[[:lower:]]$",
        // A flag after an escaped parenthesis and after a class that holds
        // a bracket.
        r"^\((?i)k\)$",
        r"^[\]](?i)k$",
        "^[(](?i)k$",
        // Unicode turned back on inside a fold.
        "^(?i-u:(?u:k))$",
        "^(?-u:(?i:(?u)k))$",
        "^(?i-u)k|(?u)s$",
        // Under `x`: spaces around the flag, a comment before it.
        "^(?x) k (?i) s $",
        "^(?x) k # no fold here\n (?i) s $",
        "^(?xi)k$",
        "^(?x: k (?i: s ) )$",
        "^(?x)(?i-x) k$",
        // The conservative cases: the flag stands where it folds nothing.
        "^k(?i)$",
        "^(?:k(?i))s$",
        "^((?i))k$",
        "^(?i)(?-u)k$",
        "^(?i)(?-i)k$",
        "^(?i:)k$",
    ];
    for pattern in refused {
        assert!(
            regex::Regex::new(pattern).is_ok() && reference(pattern).is_some(),
            "{pattern:?} is one the regex crate compiles"
        );
        assert_eq!(
            read(pattern).err(),
            Some(Unreadable::Malformed),
            "{pattern:?}"
        );
        assert_eq!(
            requested(pattern),
            Err(Refused::InvalidInput),
            "{pattern:?}"
        );
    }
    let taken: [(&str, &[&str], &[&str]); 14] = [
        ("^(?i-u)k$", &["k", "K"], &["\u{212a}", "kk"]),
        (
            "^(?-u)(?i)[j-l]s$",
            &["ks", "KS", "jS"],
            &["\u{212a}s", "k\u{17f}"],
        ),
        ("^(?-u:(?i)k)s$", &["ks", "Ks"], &["kS", "\u{212a}s"]),
        ("^(?i-u)k(?-i)s$", &["ks", "Ks"], &["kS", "\u{212a}s"]),
        (
            "^(?i-u:k)|(?-u:(?i)s)$",
            &["k", "K", "s", "S"],
            &["\u{212a}", "\u{17f}", "ks"],
        ),
        (r"^(?i-u)\x6B$", &["k", "K"], &["\u{212a}"]),
        ("^(?i-u)[[:alpha:]]$", &["k", "K"], &["\u{212a}", "\u{17f}"]),
        (r"^(?i-u)\w$", &["k", "K", "_"], &["\u{212a}", "\u{e9}"]),
        // The flag of the outer group is back after the inner one.
        ("^(?i-u:k(?-i:s)k)$", &["ksk", "KsK"], &["kSk"]),
        // Under `x` a flag in a comment is no flag.
        ("^(?x)k # (?i)\n$", &["k"], &["K", "k # (?i)\n"]),
        ("^(?x: k # (?i:\n)s$", &["ks"], &["Ks", "kS"]),
        // Texts that only look like the flag.
        ("^[(?i)k]$", &["k", "i", "("], &["K", "I"]),
        (r"^\(\?i\)k$", &["(?i)k"], &["(?i)K", "k"]),
        ("^(?P<i>k)(?<u>s)$", &["ks"], &["KS"]),
    ];
    for (pattern, admitted, outside) in taken {
        assert_eq!(requested(pattern), Ok(()), "{pattern:?}");
        let grants = read(pattern).unwrap_or_else(|refusal| panic!("{pattern:?}: {refusal:?}"));
        for value in admitted {
            assert!(admits(&grants, value), "{pattern:?} on {value:?}");
        }
        for value in outside {
            assert!(!admits(&grants, value), "{pattern:?} on {value:?}");
        }
    }
}

/// What the form the texts name for a case fold, `(?i-u)`, does to the rest
/// of a pattern. The flag holds to the end of its group, so with Unicode off
/// a `.` or a negated class behind it matches a byte that is no UTF-8 and the
/// pattern is refused, as the `regex` crate refuses it; turning Unicode back
/// on while the fold holds is refused by the rule; `\d` and `\w` behind it
/// take ASCII only. The group form `(?i-u:...)`, or `(?u-i)` behind the folded
/// part, keeps the rest as it reads without the flag.
#[test]
fn a_fold_written_as_a_flag_turns_unicode_off_for_the_rest_of_the_pattern() {
    for pattern in [
        "^(?i-u)dev-.*$",
        "^(?i-u)release review of (?s:.*)$",
        "^(?i-u)dev-[^ ]+$",
        r"^(?i-u)dev-\W$",
    ] {
        assert!(regex::Regex::new(pattern).is_err(), "{pattern:?}");
        assert_eq!(
            read(pattern).err(),
            Some(Unreadable::Malformed),
            "{pattern:?}"
        );
        assert_eq!(
            requested(pattern),
            Err(Refused::InvalidInput),
            "{pattern:?}"
        );
    }
    // The `regex` crate compiles this one; the rule refuses it.
    let pattern = "^(?i-u)dev(?u)-.*$";
    assert!(regex::Regex::new(pattern).is_ok() && reference(pattern).is_some());
    assert_eq!(read(pattern).err(), Some(Unreadable::Malformed));
    assert_eq!(requested(pattern), Err(Refused::InvalidInput));

    let taken: [(&str, &[&str], &[&str]); 5] = [
        (
            "^(?i-u:dev)-.*$",
            &["DEV-12", "dev-\u{661}\u{662}"],
            &["dev", "\u{212a}-1"],
        ),
        (
            "^(?i-u)dev(?u-i)-.*$",
            &["DEV-12", "dev-\u{661}\u{662}"],
            &["dev"],
        ),
        (r"^(?i-u)dev-\d+$", &["DEV-12"], &["dev-\u{661}\u{662}"]),
        (r"^(?i-u)dev-\w+$", &["DEV-a1"], &["dev-\u{e9}"]),
        // A letter outside ASCII is not folded.
        ("^(?i-u)\u{e9}$", &["\u{e9}"], &["\u{c9}"]),
    ];
    for (pattern, admitted, outside) in taken {
        assert_eq!(requested(pattern), Ok(()), "{pattern:?}");
        let grants = read(pattern).unwrap_or_else(|refusal| panic!("{pattern:?}: {refusal:?}"));
        for value in admitted {
            assert!(admits(&grants, value), "{pattern:?} on {value:?}");
        }
        for value in outside {
            assert!(!admits(&grants, value), "{pattern:?} on {value:?}");
        }
    }
}

/// Invariant 4: the matcher built from the lazy DFA and the PikeVM answers
/// as `regex::Regex` does on `\A(?:pattern)\z`, over the shared vectors and
/// over patterns whose answer comes from different engines: a Unicode word
/// boundary beside a byte over 0x7f (the lazy DFA quits), a counted
/// repetition near the size limit (its cache fills), line and CRLF anchors,
/// lazy and swapped greediness, empty branches.
#[test]
fn the_matcher_answers_as_the_regex_crate_does_on_the_wrapped_pattern() {
    let shared: Value =
        serde_json::from_str(include_str!("../fixtures/whole_value_patterns.json")).unwrap();
    let shared = shared["vectors"].as_array().unwrap();
    assert!(shared.len() >= 30, "{}", shared.len());
    let shared_values: Vec<String> = shared
        .iter()
        .map(|vector| vector["value"].as_str().unwrap().to_owned())
        .collect();
    let mut cases: Vec<(String, Vec<String>)> = shared
        .iter()
        .filter(|vector| !vector["matches"].is_null())
        .map(|vector| {
            (
                vector["pattern"].as_str().unwrap().to_owned(),
                shared_values.clone(),
            )
        })
        .collect();
    let x = |count: usize| "x".repeat(count);
    let own: Vec<(&str, Vec<String>)> = vec![
        (
            r"^\ba?x{2034}\b$",
            vec![
                x(2034),
                x(2033),
                x(2035),
                format!("a{}", x(2034)),
                format!("{}\u{e9}", x(2032)),
                format!("{}\u{e9}", x(2033)),
                format!("{} ", x(2033)),
            ],
        ),
        (
            r"^\b\w+\b$",
            vec![
                "\u{e9}".into(),
                "a\u{e9}b".into(),
                "a b".into(),
                "\u{663}".into(),
            ],
        ),
        (
            r"^[a-z\x{e9}]+\b \b[a-z\x{e9}]+$",
            vec![
                "a b".into(),
                "\u{e9} \u{e9}".into(),
                "a  b".into(),
                "a\u{a0}b".into(),
            ],
        ),
        (r"^\B-\B$", vec!["-".into(), "a-".into(), "\u{e9}-".into()]),
        (
            "^(?m:^a$\n^b$)$",
            vec!["a\nb".into(), "a\nb\n".into(), "a\r\nb".into()],
        ),
        (
            "^(?Rm:^a$\r\n^b$)$",
            vec!["a\r\nb".into(), "a\nb".into(), "a\r\nb\r".into()],
        ),
        ("^(?R)a.$", vec!["a\r".into(), "a\n".into(), "ab".into()]),
        ("^(?s:.)*?b$", vec!["b".into(), "a\nb".into(), "ba".into()]),
        ("^(?U)a+b*$", vec!["a".into(), "aab".into(), "b".into()]),
        (
            "^(a|ab)(c|bcd)(d*)$",
            vec!["abcd".into(), "abcdd".into(), "ab".into()],
        ),
        ("^(|a)+$", vec!["".into(), "aaa".into(), "b".into()]),
        ("^(?:)$", vec!["".into(), " ".into()]),
        (
            "^[ab]*a[ab]{40}$",
            vec![
                "a".repeat(41),
                "b".repeat(41),
                format!("a{}", "b".repeat(40)),
                format!("ab{}", "b".repeat(40)),
                "ab".repeat(400),
                format!("{}a{}", "b".repeat(700), "b".repeat(40)),
            ],
        ),
        ("^(?:x?){600}$", vec![x(0), x(1), x(600), x(601)]),
        ("^[a-z]{906}$", vec!["a".repeat(906), "a".repeat(905)]),
        (
            r"^\pL+\d*$",
            vec![
                "\u{e9}\u{663}".into(),
                "a1".into(),
                "1".into(),
                "\u{3b1}".into(),
            ],
        ),
        (
            "^(?i-u)dev-[0-9]+$",
            vec!["DEV-1".into(), "dEv-12".into(), "DEV-\u{661}".into()],
        ),
    ];
    cases.extend(
        own.into_iter()
            .map(|(pattern, values)| (pattern.to_owned(), values)),
    );
    let mut random = Random(0x5eed_5eed_5eed_5eed);
    let (mut compared, mut matched) = (0, 0);
    for (pattern, bases) in cases {
        let grants = read(&pattern).unwrap_or_else(|refusal| panic!("{pattern:?}: {refusal:?}"));
        let whole = reference(&pattern).unwrap_or_else(|| panic!("{pattern:?}"));
        let mut values = bases.clone();
        let mut letters: Vec<char> = vec!['\n', '\r', '\u{e9}', 'x', ' '];
        for base in &bases {
            letters.extend(base.chars().take(40));
            let mut shorter = base.clone();
            shorter.pop();
            values.extend([
                format!("{base}\n"),
                format!("\n{base}"),
                format!("{base}x"),
                format!("{base}\r"),
                format!("{base}\u{e9}"),
                base.to_uppercase(),
                base.to_lowercase(),
                shorter,
            ]);
        }
        for _ in 0..60 {
            let length = random.below(13);
            values.push((0..length).map(|_| random.pick(&letters)).collect());
        }
        // Changed in fix round 2 (the work bound): the reader answers as
        // `regex::Regex` does on every value of at most the pattern's
        // `max_value_bytes` and admits no longer one (it asserted the same
        // answer on every value; `^[a-z]{906}$` and `^(?:x?){600}$` are
        // matched against fewer bytes than their longest value here).
        let bound = longest(&file_of(&pattern, 1));
        for value in values {
            let answer = admits(&grants, &value);
            assert_eq!(
                answer,
                value.len() <= bound && whole.is_match(&value),
                "{pattern:?} on {value:?}: the reader and regex::Regex differ \
                 (the pattern is matched against {bound} bytes)"
            );
            compared += 1;
            matched += usize::from(answer);
        }
    }
    println!("{compared} answers compared, {matched} of them a match");
    assert!(matched >= 100, "{matched}");
}

/// One read of `file` and the check of the write `{"k": value}`: the bytes
/// this thread holds once the write was checked, the grants still held, and
/// how many grants cover the write.
fn one_read(file: &[u8], value: &str) -> (isize, usize) {
    let input = json!({"k": value});
    let before = live();
    let grants = standing::parse(file).expect("a file within the stated limits");
    let covering = grants
        .covering(INSTANCE, OPERATION, &input, 1_000)
        .map_or(0, |covering| covering.len());
    let holds = live() - before;
    drop(grants);
    (holds, covering)
}

/// The contract's costliest pattern: 113 times one class of 64 byte ranges,
/// the characters of odd code point from U+0001 to U+007F (fix round 2).
const COSTLIEST: &str = r"^[\x01\x03\x05\x07\x09\x0b\x0d\x0f\x11\x13\x15\x17\x19\x1b\x1d\x1f!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{}\x7f]{113}$";

/// The measured figures of contracts/service/compatibility.md § 2.1 that the
/// first review's file does not check: "`^\ba?x{2034}\b$` checked against a
/// value of 2,034 times `x`, holds 336,513 bytes as the one member of a
/// file", "64 members of `^\b\w\b$` hold 6,579,638 bytes, of
/// `^Release review of (?s:.*)$` 1,670,774 bytes and of `^DEV-[0-9]+$`
/// 927,670 bytes". Each within one part in a hundred.
///
/// Changed in fix round 2 together with the contract's text (the reader
/// matches with the PikeVM alone): the costliest pattern is 113 times one
/// class of 64 byte ranges, checked against 113 times `a`, and holds 61,727
/// bytes as the one member of a file; 64 members of `^\b\w\b$` hold
/// 1,194,550 bytes, of `^Release review of (?s:.*)$` 113,910 bytes and of
/// `^DEV-[0-9]+$` 76,086 bytes. The band is as it was.
#[test]
fn the_other_measured_figures_of_the_contract_hold() {
    let a = "a".repeat(113);
    for (pattern, value, members, stated) in [
        (COSTLIEST, a.as_str(), 1, 61_727_isize),
        (r"^\b\w\b$", "a", 64, 1_194_550),
        (
            "^Release review of (?s:.*)$",
            "Release review of 2.46.8",
            64,
            113_910,
        ),
        ("^DEV-[0-9]+$", "DEV-12", 64, 76_086),
    ] {
        let (holds, covering) = one_read(&file_of(pattern, members), value);
        assert_eq!(covering, members, "{pattern:?}");
        println!("{members} members of {pattern:?} hold {holds} bytes; stated {stated}");
        assert!(
            (stated - stated / 100..=stated + stated / 100).contains(&holds),
            "{members} members of {pattern:?} hold {holds} bytes; the contract states {stated}"
        );
    }
}

/// The CPU time of one read of `file` and of the check of the write
/// `{"k": value}` against it, and how many grants cover the write.
fn one_checked_write(file: &[u8], value: &str) -> (Duration, Duration, usize) {
    let input = json!({"k": value});
    let before = cpu();
    let grants = standing::parse(file).expect("a file within the stated limits");
    let read = cpu() - before;
    let before = cpu();
    let covering = grants
        .covering(INSTANCE, OPERATION, &input, 1_000)
        .map_or(0, |covering| covering.len());
    (read, cpu() - before, covering)
}

/// The `max_value_bytes` of the first target member of `file`, a pattern:
/// the longest value the reader matches it against (fix round 2).
fn longest(file: &[u8]) -> usize {
    let grants = standing::parse(file).expect("a file the reader takes");
    match &grants.grants()[0].target[0].bound {
        standing::Bound::Pattern(pattern) => pattern.max_value_bytes(),
        other => panic!("not a pattern member: {other:?}"),
    }
}

/// The control of the next test: the two patterns of the consumer, 64
/// members each, check a value as large as a request can carry (60,000
/// bytes of a 64 KiB request) in under a second of CPU time.
///
/// Changed in fix round 2 (the work bound): a pattern member is matched
/// against no value longer than its `max_value_bytes`, so the request-sized
/// value is inside no grant (it asserted 64 covering grants), still within
/// the second. The value of exactly `max_value_bytes`, at least 8,192 bytes
/// for either pattern, is inside all 64 within the second.
#[test]
fn the_documented_patterns_check_a_request_sized_value_within_a_second() {
    for (pattern, head, unit, pad) in [
        ("^DEV-[0-9]+$", "DEV-", "1", "1"),
        (
            "^Release review of (?s:.*)$",
            "Release review of ",
            "line \u{e9}\n",
            "\n",
        ),
    ] {
        let file = file_of(pattern, 64);
        let bound = longest(&file);
        assert!(bound >= 8_192, "{pattern:?}: {bound}");
        let large = format!("{head}{}", unit.repeat(60_000 / unit.len()));
        assert!(large.len() >= 60_000, "{}", large.len());
        let (read, checked, covering) = one_checked_write(&file, &large);
        assert_eq!(covering, 0, "{pattern:?} on {} bytes", large.len());
        assert!(read + checked < Duration::from_secs(1), "{pattern:?}");
        let mut at = format!("{head}{}", unit.repeat((bound - head.len()) / unit.len()));
        at.push_str(&pad.repeat(bound - at.len()));
        assert_eq!(at.len(), bound, "{pattern:?}");
        let (read, checked, covering) = one_checked_write(&file, &at);
        assert_eq!(covering, 64, "{pattern:?} on {bound} bytes");
        println!("64 members of {pattern:?} on {bound} bytes: read {read:?}, check {checked:?}");
        assert!(read + checked < Duration::from_secs(1), "{pattern:?}");
    }
}

/// Invariant 6, time, on the half no stated limit bounds: the check of the
/// write. A file within every stated limit (64 pattern members, each of 11
/// bytes and accepted under the compiled-size limit) and a value of 4,000
/// bytes, a sixteenth of what a 64 KiB request carries. Under `^x*x{2000}$`
/// a value of `x` keeps every state of the program live, the lazy DFA gives
/// its cache up, and the PikeVM steps 2,000 states for each byte of the
/// value, for each of the 64 members. The bound is the one acceptance 25
/// gives the read: under a second, here of this thread's CPU time.
///
/// Changed in fix round 2 (the work bound): the reader matches a pattern
/// member against no value longer than its `max_value_bytes`, which is under
/// 4,000 bytes for this pattern, so the write is inside no grant (it
/// asserted 64 covering grants). The bound on the time stays, and holds for
/// the value of exactly `max_value_bytes` too, and for a pattern of the same
/// family whose longest value matches it (`^x*x{150}$`: all 64 cover).
#[test]
fn one_checked_write_against_a_file_within_the_stated_limits_takes_under_a_second() {
    let pattern = "^x*x{2000}$";
    let file = file_of(pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let bound = longest(&file);
    assert!(bound < 4_000, "{pattern} is matched against {bound} bytes");
    for value in ["x".repeat(4_000), "x".repeat(bound)] {
        let (read, checked, covering) = one_checked_write(&file, &value);
        assert_eq!(covering, 0, "{} bytes of x under {pattern}", value.len());
        assert!(
            checked < Duration::from_secs(1),
            "the check of one write of a {}-byte value against a {}-byte file with 64 members \
             of {pattern} takes {checked:?} of CPU time (the read before it {read:?})",
            value.len(),
            file.len(),
        );
    }
    let pattern = "^x*x{150}$";
    let file = file_of(pattern, 64);
    let value = "x".repeat(longest(&file));
    let (read, checked, covering) = one_checked_write(&file, &value);
    assert_eq!(covering, 64, "the write is inside every grant");
    println!(
        "64 members of {pattern} on {} bytes: read {read:?}, check {checked:?}",
        value.len()
    );
    assert!(
        checked < Duration::from_secs(1),
        "the check of one write of a {}-byte value against 64 members of {pattern} takes \
         {checked:?} of CPU time",
        value.len()
    );
}
