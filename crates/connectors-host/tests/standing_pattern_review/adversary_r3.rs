//! Round 3 adversary review of
//! story:standing-grant-binds-a-pattern-or-a-value-set, after fix round 2:
//! the reader of `local::standing` matches a pattern member with the PikeVM
//! of `regex-automata` alone and against no value longer than the member's
//! `max_value_bytes`, the work constant over the weight of the pattern's
//! program (contracts/service/compatibility.md § 2.1;
//! contracts/cli/v1alpha1/semantics.md, "Standing grants";
//! docs/local-approvals.md, "A grant for many targets").
//!
//! Driven through the public surface: `standing::parse`, `Grants::covering`,
//! `standing::check_request`, `Pattern` and `TargetMember::listed`.
//!
//! - What a check costs against what the weight says, for a Unicode word
//!   boundary beside a character that is no word character.
//! - The bound at its edges: bytes and not characters, the value of exactly
//!   the bound and one byte more, two members of one grant, a value set.
//! - The reader against `regex::Regex` on the wrapped pattern, for every
//!   value within the bound.
//! - `approvals grant-standing` and the reader on the same patterns, and
//!   what a member the command would not sign covers in a file.
//! - What `approvals list-standing` shows.
//! - The figures the texts state against what the reader computes.
#![cfg(unix)]
use crate::support::{LIVE, PEAK};
use connectors_host::local::standing::{
    self, Bound, Grants, NotAdmitted, Pattern, Refused, Uncovered, Unreadable,
};
use serde_json::{Value, json};
use std::{cell::Cell, time::Duration};

const INSTANCE: &str = "leaf";
const OPERATION: &str = "issue.transition.run";
const LATER: i64 = 2_000_000_000_000;

const DEV: &str = "^DEV-[0-9]+$";
const RELEASE: &str = "^Release review of (?s:.*)$";
const SK: &str = "^(?i-u:sk)-[0-9]+$";
/// The pattern the texts name as one of the costliest found.
const LIVE_RANGES: &str = "^x*x{150}$";

fn grant(id: &str, target: Value) -> Value {
    json!({
        "id": id, "instance": INSTANCE, "operation": OPERATION,
        "target": target,
        "not_after_unix_ms": LATER, "max_per_hour": 1
    })
}

fn document(grants: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"format": standing::FORMAT, "grants": grants, "revoked": []}))
        .unwrap()
}

/// A grant file of `members` grants, each with the one target member
/// `{"pointer": "/k", "pattern": pattern}`.
fn file_of(pattern: &str, members: usize) -> Vec<u8> {
    let grants: Vec<Value> = (0..members)
        .map(|n| {
            grant(
                &format!("g{n}"),
                json!([{"pointer": "/k", "pattern": pattern}]),
            )
        })
        .collect();
    document(json!(grants))
}

/// What the check of `approvals grant-standing` answers for a request whose
/// one target member holds `pattern`.
fn requested(pattern: &str) -> Result<(), Refused> {
    let request = serde_json::to_vec(&json!({
        "operation": OPERATION,
        "target": [{"pointer": "/k", "pattern": pattern}],
        "not_after_unix_ms": LATER, "max_per_hour": 1
    }))
    .unwrap();
    standing::check_request(&request, INSTANCE)
}

/// The file of one member that holds `pattern`, as the reader takes it.
#[track_caller]
fn read(pattern: &str) -> Grants {
    standing::parse(&file_of(pattern, 1))
        .unwrap_or_else(|refusal| panic!("{pattern}: the reader answers {refusal:?}"))
}

/// The `max_value_bytes` of the first target member of the first grant.
#[track_caller]
fn bound_of(grants: &Grants) -> usize {
    match &grants.grants()[0].target[0].bound {
        Bound::Pattern(pattern) => pattern.max_value_bytes(),
        other => panic!("not a pattern member: {other:?}"),
    }
}

/// How many grants cover the write `input`, or why none does.
fn covering(grants: &Grants, input: &Value) -> Result<usize, Uncovered> {
    grants
        .covering(INSTANCE, OPERATION, input, 1_000)
        .map(|covering| covering.len())
}

fn covers(grants: &Grants, value: &str) -> Result<usize, Uncovered> {
    covering(grants, &json!({"k": value}))
}

// ---------------------------------------------------------------------------
// 1. What a check costs against what the weight says.
// ---------------------------------------------------------------------------

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

/// One check of a file of 64 members of `pattern` against a run of `letter`
/// of the member's `max_value_bytes` (the most whole letters within it): the
/// value's length, how many grants cover the write, and the CPU time of the
/// check alone.
fn checked_at_its_bound(pattern: &str, letter: &str) -> (usize, usize, Duration) {
    let file = file_of(pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let grants = standing::parse(&file)
        .unwrap_or_else(|refusal| panic!("{pattern}: the reader answers {refusal:?}"));
    let value = letter.repeat(bound_of(&grants) / letter.len());
    let input = json!({"k": value});
    let started = cpu();
    let covering = covering(&grants, &input).unwrap_or(0);
    let took = cpu() - started;
    (value.len(), covering, took)
}

/// A run of `-`, then `looks` Unicode word boundaries (`\B`) before one more
/// `-`: after every byte of a run of `-` the PikeVM evaluates every one of
/// them, and each looks up the character before and the character after in
/// the table of word characters, since `-` is no ASCII word byte.
fn boundaries_in_a_run_of_dashes(looks: usize) -> String {
    format!("^-*{}-$", r"\B".repeat(looks))
}

/// The same with the looks spread over `times` repetitions.
fn boundaries_repeated(looks: usize, times: usize) -> String {
    format!("^-*(?:{}-){{{times}}}$", r"\B".repeat(looks))
}

/// compatibility.md § 2.1: "one check costs at most the program's weight
/// times the value's length, which `max_value_bytes` bounds", measured as
/// 157 ms to 399 ms for "the costliest patterns found". The second a checked
/// write may take, as the suite's own cases assert it, for a file
/// `approvals grant-standing` signs: 64 members whose pattern is a run of
/// Unicode word boundaries between characters that are no word characters,
/// each checked on a value of exactly its `max_value_bytes`.
#[test]
fn a_word_boundary_between_characters_outside_the_word_class_is_checked_within_a_second() {
    let multiplication = '\u{d7}';
    let cases = [
        (boundaries_repeated(8, 10), "-".to_owned()),
        (boundaries_repeated(60, 2), "-".to_owned()),
        (boundaries_in_a_run_of_dashes(120), "-".to_owned()),
        (
            format!(
                "^{multiplication}*(?:{}{multiplication}){{10}}$",
                r"\B".repeat(8)
            ),
            multiplication.to_string(),
        ),
    ];
    let mut over = Vec::new();
    for (pattern, letter) in &cases {
        assert!(pattern.len() <= 256, "{} bytes", pattern.len());
        assert_eq!(requested(pattern), Ok(()), "{pattern}: grant-standing");
        let (bytes, covering, first) = checked_at_its_bound(pattern, letter);
        let (_, _, second) = checked_at_its_bound(pattern, letter);
        assert_eq!(covering, 64, "{pattern} on {bytes} bytes of {letter:?}");
        let least = first.min(second);
        let line = format!(
            "64 checks of {pattern} on {bytes} bytes each: {least:?} of CPU time at the least \
             ({first:?}, {second:?})"
        );
        println!("{line}");
        if least >= Duration::from_secs(1) {
            over.push(line);
        }
    }
    assert!(
        over.is_empty(),
        "one check of a file within every stated limit, each member on a value of its \
         max_value_bytes, takes a second or more: {over:#?}"
    );
}

/// `local::standing`, on the weights: "with them no pattern tried costs more
/// per unit than a run of byte ranges that are all live (`^x*x{150}$`)", and
/// compatibility.md § 2.1 names that pattern among "the costliest patterns
/// found". Both files are checked at the bound of their own pattern, so both
/// ask for the same work by the formula; the times are the least of three
/// checks each, taken in turn in this one thread. A Unicode word boundary
/// weighs 32, four states; between two characters that are no ASCII word
/// bytes it costs two searches of the table of word characters.
#[test]
fn a_word_boundary_costs_no_more_per_unit_of_weight_than_a_run_of_live_byte_ranges() {
    let heavy = boundaries_in_a_run_of_dashes(120);
    let fast = format!("^x*{}x$", r"\B".repeat(120));
    let accented = format!("^\u{e9}*{}\u{e9}$", r"\B".repeat(120));
    let cases = [
        (LIVE_RANGES, "x"),
        (heavy.as_str(), "-"),
        (fast.as_str(), "x"),
        (accented.as_str(), "\u{e9}"),
    ];
    let mut least = [Duration::MAX; 4];
    let mut bytes = [0; 4];
    for _ in 0..3 {
        for (at, (pattern, letter)) in cases.iter().enumerate() {
            let (length, covering, took) = checked_at_its_bound(pattern, letter);
            assert_eq!(covering, 64, "{pattern} on {length} bytes of {letter:?}");
            least[at] = least[at].min(took);
            bytes[at] = length;
        }
    }
    for (at, (pattern, _)) in cases.iter().enumerate() {
        println!(
            "64 checks of {pattern} on {} bytes each: {:?} of CPU time at the least, {:.2} times \
             {LIVE_RANGES}",
            bytes[at],
            least[at],
            least[at].as_secs_f64() / least[0].as_secs_f64()
        );
    }
    let ratio = least[1].as_secs_f64() / least[0].as_secs_f64();
    assert!(
        ratio <= 1.25,
        "64 checks of {heavy} on {} bytes each take {:?} of CPU time, {ratio:.2} times the {:?} \
         of {LIVE_RANGES} on {} bytes each, which the texts name as the costliest found; both \
         are checked at their own max_value_bytes",
        bytes[1],
        least[1],
        least[0],
        bytes[0]
    );
}

// ---------------------------------------------------------------------------
// 2. The bound at its edges.
// ---------------------------------------------------------------------------

/// compatibility.md § 2.1: "`max_value_bytes`", "a string longer than that is
/// covered by no grant through that member, whatever the pattern would
/// answer", "the host's log names the reason". The bound counts the bytes of
/// the value: a value of characters of two, three and four bytes is inside
/// at the bound and outside one character later, when it still holds fewer
/// characters than the bound. The module's own cases use ASCII values only.
#[test]
fn the_bound_counts_the_bytes_of_a_value_and_not_its_characters() {
    let grants = read(RELEASE);
    let bound = bound_of(&grants);
    let head = "Release review of ";
    for letter in ["\u{e9}", "\u{20ac}", "\u{1f600}"] {
        let at = format!(
            "{head}{}",
            letter.repeat((bound - head.len()) / letter.len())
        );
        let over = format!("{at}{letter}");
        assert!(at.len() <= bound && bound - at.len() < letter.len());
        assert!(over.len() > bound && over.chars().count() < bound);
        assert_eq!(
            covers(&grants, &at),
            Ok(1),
            "{} bytes of {letter:?} under a bound of {bound}",
            at.len()
        );
        assert_eq!(
            covers(&grants, &over),
            Err(Uncovered::TooLong),
            "{} bytes in {} characters of {letter:?} under a bound of {bound}",
            over.len(),
            over.chars().count()
        );
    }
    // Exactly the bound in two-byte characters, and the empty value.
    let exact = format!("{head}{}", "\u{e9}".repeat((bound - head.len()) / 2));
    assert_eq!(exact.len(), bound);
    assert_eq!(covers(&grants, &exact), Ok(1));
    assert_eq!(covers(&grants, ""), Err(Uncovered::OtherTarget));
    assert_eq!(covers(&read("^(?s:.*)$"), ""), Ok(1));
    // The log's line for the reason is its own.
    let too_long = NotAdmitted::Uncovered(Uncovered::TooLong).event();
    assert_ne!(
        too_long,
        NotAdmitted::Uncovered(Uncovered::OtherTarget).event()
    );
    assert!(too_long.contains("longer"), "{too_long}");
}

/// `Pattern::matches`, a public function: "A value longer than
/// `Pattern::max_value_bytes` does not: the program is not run on it."
#[test]
fn a_pattern_asked_directly_matches_no_value_longer_than_its_bound() {
    let pattern = Pattern::new(DEV.to_owned()).expect("a pattern of the guide");
    let bound = pattern.max_value_bytes();
    let at = format!("DEV-{}", "1".repeat(bound - 4));
    assert_eq!(at.len(), bound);
    assert!(pattern.matches(&at), "{bound} bytes");
    assert!(!pattern.matches(&format!("{at}1")), "{} bytes", bound + 1);
    let reference = regex::Regex::new(&format!(r"\A(?:{DEV})\z")).unwrap();
    assert!(reference.is_match(&format!("{at}1")));
}

/// Two pattern members of one grant are each held to their own bound, and a
/// value-set member beside them to none: `one_of` is "compared as `value`
/// is", whatever the length of the string.
#[test]
fn two_pattern_members_of_one_grant_are_each_held_to_their_own_bound() {
    let long = "s".repeat(30_000);
    let file = document(json!([grant(
        "g1",
        json!([
            {"pointer": "/a", "pattern": DEV},
            {"pointer": "/b", "pattern": RELEASE},
            {"pointer": "/s", "one_of": ["short", long]}
        ])
    )]));
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let grants = standing::parse(&file).expect("a file within the limits");
    let bounds: Vec<usize> = grants.grants()[0]
        .target
        .iter()
        .filter_map(|member| match &member.bound {
            Bound::Pattern(pattern) => Some(pattern.max_value_bytes()),
            _ => None,
        })
        .collect();
    let [dev, release] = bounds[..] else {
        panic!("{bounds:?}")
    };
    assert!(dev > release + 1, "{dev} and {release}");
    let a = |bytes: usize| format!("DEV-{}", "1".repeat(bytes - 4));
    let b = |bytes: usize| format!("Release review of {}", "x".repeat(bytes - 18));
    let write = |a: String, b: String, s: &str| json!({"a": a, "b": b, "s": s});
    assert_eq!(
        covering(&grants, &write(a(dev), b(release), &long)),
        Ok(1),
        "each at its bound"
    );
    assert_eq!(
        covering(&grants, &write(a(release + 1), b(release), "short")),
        Ok(1),
        "/a is longer than the bound of /b"
    );
    assert_eq!(
        covering(&grants, &write(a(dev + 1), b(release), "short")),
        Err(Uncovered::TooLong)
    );
    assert_eq!(
        covering(&grants, &write(a(dev), b(release + 1), "short")),
        Err(Uncovered::TooLong)
    );
    assert_eq!(
        covering(&grants, &write(a(dev), b(release), &format!("{long}s"))),
        Err(Uncovered::OtherTarget)
    );
}

/// compatibility.md § 2.1, of a string longer than a member's
/// `max_value_bytes`: "the write is `approval_required` and the host's log
/// names the reason". The log holds one reason for a write, the most
/// specific over every grant of the operation. A file with one grant whose
/// pattern the value is too long for and one grant whose exact value it is
/// not: the reason is the length, in either file order, so the operator
/// reads why the pattern grant did not cover the write. A grant that admits
/// the long string as an exact value covers it.
#[test]
fn a_value_too_long_for_one_grant_is_named_so_beside_a_grant_it_is_outside() {
    let bound = bound_of(&read(DEV));
    let over = format!("DEV-{}", "1".repeat(bound - 3));
    assert_eq!(over.len(), bound + 1);
    let by_pattern = || grant("g1", json!([{"pointer": "/k", "pattern": DEV}]));
    let by_value = || grant("g2", json!([{"pointer": "/k", "value": "DEV-7"}]));
    for (order, grants) in [
        ("pattern first", json!([by_pattern(), by_value()])),
        ("value first", json!([by_value(), by_pattern()])),
    ] {
        let grants = standing::parse(&document(grants)).expect("a file within the limits");
        assert_eq!(covers(&grants, &over), Err(Uncovered::TooLong), "{order}");
        assert_eq!(covers(&grants, "DEV-7"), Ok(2), "{order}");
        assert_eq!(
            covers(&grants, "PROD-7"),
            Err(Uncovered::OtherTarget),
            "{order}"
        );
    }
    let whole = grant("g3", json!([{"pointer": "/k", "one_of": [over]}]));
    let grants = standing::parse(&document(json!([by_pattern(), by_value(), whole])))
        .expect("a file within the limits");
    assert_eq!(covers(&grants, &over), Ok(1));
}

// ---------------------------------------------------------------------------
// 3. The reader against `regex::Regex` on the wrapped pattern.
// ---------------------------------------------------------------------------

/// A fixed-seed generator (xorshift64).
struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// compatibility.md § 2.1: a pattern "is matched as `\A(?:pattern)\z`", and
/// the reader's own text: "No answer changes: every engine decides the same
/// match." For every pattern here, look-around in each of its forms among
/// them, and every value within the pattern's bound, the reader answers as
/// `regex::Regex` does on the wrap; a value over the bound is `TooLong`.
#[test]
fn the_reader_answers_as_the_regex_crate_does_for_every_value_within_the_bound() {
    let patterns = [
        boundaries_repeated(8, 10),
        boundaries_in_a_run_of_dashes(120),
        format!("^x*{}x$", r"\B".repeat(120)),
        LIVE_RANGES.to_owned(),
        "^(?s:.*)a?x{150}$".to_owned(),
        "^(?i-u:x*x{150})$".to_owned(),
        r"^(?:\b|\B)x$".to_owned(),
        r"^(?:\B*x*){5}$".to_owned(),
        r"^(?:(?:\b|\B|\b|\B|\b|\B)*x*){5}$".to_owned(),
        r"^\b{start}x+\b{end}$".to_owned(),
        r"^\b{start-half}[ax-]*\b{end-half}$".to_owned(),
        r"^(?-u:\b)[ax-]+(?-u:\b)$".to_owned(),
        r"^-*\b[a-c]+\b-*$".to_owned(),
        "^(?m:^a$\n?)*$".to_owned(),
        r"^(?m:^[ab]*$\n?)*$".to_owned(),
        r"^(?m)a$\n^b$".to_owned(),
        r"^(?Rm:^a$\r\n)*$".to_owned(),
        r"^(?Rm:^[ab ]*$(?:\r\n|\r|\n)?)*$".to_owned(),
        r"^\pL+$".to_owned(),
        r"^(?i-u:[a-c]+)x$".to_owned(),
        "^(?:a{2,5}|b?){0,7}$".to_owned(),
        "^(?:|a)(?:b|)$".to_owned(),
        "^.*$".to_owned(),
        "^(?s:.)*$".to_owned(),
        "^[^-]*$".to_owned(),
        "^\u{1f600}*\u{e9}?$".to_owned(),
        DEV.to_owned(),
        RELEASE.to_owned(),
        SK.to_owned(),
    ];
    let atoms = [
        "a",
        "b",
        "c",
        "x",
        "-",
        " ",
        "\n",
        "\r",
        "\r\n",
        "\u{e9}",
        "\u{d7}",
        "A",
        "B",
        "K",
        "\u{212a}",
        "\u{1f600}",
        "1",
        "_",
    ];
    let mut generator = Generator(0x9e37_79b9_7f4a_7c15);
    let mut generated: Vec<String> = (0..400)
        .map(|_| {
            (0..generator.below(13))
                .map(|_| atoms[generator.below(atoms.len())])
                .collect()
        })
        .collect();
    for letter in ["x", "-", "a", "\u{e9}"] {
        for times in [0, 1, 2, 3, 9, 10, 11, 12, 121, 122, 150, 151, 152] {
            generated.push(letter.repeat(times));
        }
    }
    let (mut taken, mut compared, mut matched) = (0, 0, 0);
    for pattern in &patterns {
        let reference = regex::Regex::new(&format!(r"\A(?:{pattern})\z"));
        let grants = match (standing::parse(&file_of(pattern, 1)), &reference) {
            (Ok(grants), Ok(_)) => grants,
            (Err(Unreadable::Malformed), Err(_)) => continue,
            (file, _) => panic!(
                "{pattern:?}: the reader answers {:?} and the regex crate compiles it: {}",
                file.map(drop),
                reference.is_ok()
            ),
        };
        let reference = reference.unwrap();
        taken += 1;
        let bound = bound_of(&grants);
        let mut values = generated.clone();
        for letter in ["x", "-", "\u{e9}"] {
            let most = bound / letter.len();
            values.push(letter.repeat(most));
            values.push(letter.repeat(most.saturating_sub(1)));
            values.push(letter.repeat(most + 1));
        }
        for value in &values {
            let answer = covers(&grants, value);
            if value.len() > bound {
                assert_eq!(
                    answer,
                    Err(Uncovered::TooLong),
                    "{pattern:?} on {} bytes over its bound of {bound}",
                    value.len()
                );
                continue;
            }
            let expected = reference.is_match(value);
            compared += 1;
            matched += usize::from(expected);
            assert_eq!(
                answer,
                if expected {
                    Ok(1)
                } else {
                    Err(Uncovered::OtherTarget)
                },
                "{pattern:?} on {value:?} ({} bytes, bound {bound}): the regex crate answers \
                 {expected}",
                value.len()
            );
        }
    }
    println!(
        "{taken} of {} patterns taken, {compared} values compared, {matched} of them matches",
        patterns.len()
    );
    assert!(taken >= 27, "{taken} of {} patterns taken", patterns.len());
    assert!(matched >= 400, "{matched} matches of {compared} values");
}

// ---------------------------------------------------------------------------
// 4. `approvals grant-standing` and the reader.
// ---------------------------------------------------------------------------

/// semantics.md, "Standing grants": "The command refuses, as
/// `invalid_input`, a pattern whose `max_value_bytes` is under 64 [...]. A
/// reader takes a file that holds such a member, and the member covers no
/// value longer than its own figure." The command takes a pattern exactly
/// when the reader takes it and its bound is at least 64; the edge is shown
/// on real patterns of bound 64 and 63, through the command's check.
#[test]
fn grant_standing_signs_a_pattern_exactly_when_the_reader_matches_it_against_64_bytes() {
    let looks = r"\B".repeat(8);
    let mut seen = std::collections::BTreeMap::new();
    for times in 100..=220 {
        let pattern = format!("^(?:{looks}-){{{times}}}$");
        let Ok(grants) = standing::parse(&file_of(&pattern, 1)) else {
            continue;
        };
        let bound = bound_of(&grants);
        if (62..=66).contains(&bound) {
            seen.entry(bound).or_insert(pattern);
        }
    }
    assert!(
        seen.contains_key(&64) && seen.contains_key(&63),
        "no pattern of the family has a bound of 64 and none of 63: {seen:?}"
    );
    for (bound, pattern) in &seen {
        assert_eq!(
            requested(pattern),
            if *bound >= 64 {
                Ok(())
            } else {
                Err(Refused::InvalidInput)
            },
            "{pattern} is matched against {bound} bytes"
        );
    }
    for pattern in [
        DEV,
        RELEASE,
        SK,
        LIVE_RANGES,
        "^[a-z]{906}$",
        "^[a-z]{907}$",
        r"^\w+$",
        r"^\w{2}$",
        "^(?i)dev$",
        "^(?i-u)dev$",
        "DEV-[0-9]+",
        r"^(?:\B\B\B\B\B\B\B\B-){200}$",
        r"^(?:\B-){200}$",
    ] {
        let file = standing::parse(&file_of(pattern, 1));
        let expected = match &file {
            Ok(grants) if bound_of(grants) >= 64 => Ok(()),
            Ok(_) | Err(Unreadable::Malformed) => Err(Refused::InvalidInput),
            Err(other) => panic!("{pattern}: {other:?}"),
        };
        assert_eq!(
            requested(pattern),
            expected,
            "{pattern}: the reader answers {:?}",
            file.as_ref().map(bound_of)
        );
    }
}

/// The same sentence, for a file written by hand: a member the command would
/// not sign is read, covers a matching value of up to its own figure and
/// nothing longer.
#[test]
fn a_member_the_command_would_not_sign_covers_values_up_to_its_own_bound() {
    let looks = r"\B".repeat(8);
    let (pattern, grants, bound) = (100..=190)
        .find_map(|times| {
            let pattern = format!("^(?:{looks}-){{0,{times}}}$");
            let grants = standing::parse(&file_of(&pattern, 1)).ok()?;
            let bound = bound_of(&grants);
            (bound < 64 && bound < times).then_some((pattern, grants, bound))
        })
        .expect("a pattern of the family the reader takes with a bound under 64");
    assert!(bound >= 1, "{pattern}");
    assert_eq!(requested(&pattern), Err(Refused::InvalidInput), "{pattern}");
    assert_eq!(covers(&grants, ""), Ok(1), "{pattern}");
    assert_eq!(covers(&grants, &"-".repeat(bound)), Ok(1), "{pattern}");
    assert_eq!(
        covers(&grants, &"-".repeat(bound + 1)),
        Err(Uncovered::TooLong),
        "{pattern}: bound {bound}"
    );
    assert_eq!(
        covers(&grants, &"x".repeat(bound)),
        Err(Uncovered::OtherTarget),
        "{pattern}"
    );
}

// ---------------------------------------------------------------------------
// 5. What `approvals list-standing` shows.
// ---------------------------------------------------------------------------

fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// semantics.md: "each `target` member with `pointer` and the one key it was
/// written with (`value`, `pattern` or `one_of`), and a `pattern` member also
/// with `max_value_bytes`, [...] which the command computes from the pattern
/// and the file does not hold"; ess/domains/cli.yaml, `StandingTargetView`.
/// A member of another form is shown without the key, a file or a request
/// that holds the key is refused, and the model lists the member as optional.
#[test]
fn a_listed_member_holds_max_value_bytes_exactly_when_it_is_a_pattern() {
    let target = json!([
        {"pointer": "/k", "pattern": RELEASE},
        {"pointer": "/c", "value": "C1"},
        {"pointer": "/n", "value": null},
        {"pointer": "/s", "one_of": ["a", 7, null]}
    ]);
    let grants = standing::parse(&document(json!([grant("g1", target.clone())]))).unwrap();
    let bound = bound_of(&grants);
    let listed: Vec<Value> = grants.grants()[0]
        .target
        .iter()
        .map(|member| member.listed())
        .collect();
    let mut expected = target.clone();
    expected[0]["max_value_bytes"] = json!(bound);
    assert_eq!(json!(listed), expected);
    assert_eq!(
        serde_json::to_value(&grants.grants()[0].target).unwrap(),
        target,
        "what is signed holds no such key"
    );
    // The listed form is no member of a file or of a request.
    let shown = json!([listed[0].clone()]);
    assert_eq!(
        standing::parse(&document(json!([grant("g1", shown.clone())]))).map(drop),
        Err(Unreadable::Malformed)
    );
    let request = serde_json::to_vec(&json!({
        "operation": OPERATION, "target": shown,
        "not_after_unix_ms": LATER, "max_per_hour": 1
    }))
    .unwrap();
    assert_eq!(
        standing::check_request(&request, INSTANCE),
        Err(Refused::InvalidInput)
    );
    let model = squeezed(include_str!("../../../../ess/domains/cli.yaml"));
    let view = model
        .split("- name: connectors.cli.StandingTargetView")
        .nth(1)
        .and_then(|rest| rest.split("- name: ").next())
        .expect("the model of a listed target member");
    for field in [
        "{name: pointer, type: String}",
        r#"{name: value, type: "Optional<Json>"}"#,
        r#"{name: pattern, type: "Optional<String>"}"#,
        r#"{name: one_of, type: "Optional<List<Json>>"}"#,
        r#"{name: max_value_bytes, type: "Optional<Integer>"}"#,
    ] {
        assert!(view.contains(field), "{field} in {view}");
    }
    let contract = squeezed(include_str!(
        "../../../../contracts/cli/v1alpha1/semantics.md"
    ));
    assert!(contract.contains(
        "and a `pattern` member also with `max_value_bytes`, the longest value in bytes the \
         pattern is matched against, which the command computes from the pattern and the file \
         does not hold"
    ));
}

// ---------------------------------------------------------------------------
// 7. The figures the texts state against what the reader computes.
// ---------------------------------------------------------------------------

/// `n` as the texts write a figure: `22,055`.
fn written(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Every `max_value_bytes` a text states is the figure the reader computes
/// for the pattern the text names. No case held a text to these figures: a
/// change of the work constant or of a weight left each text as it was.
#[test]
fn the_texts_state_the_bounds_the_reader_computes() {
    let bound = |pattern: &str| written(bound_of(&read(pattern)));
    let (dev, release, sk) = (bound(DEV), bound(RELEASE), bound(SK));
    let guide = format!(
        "`{DEV}` is matched against {dev} bytes, `{RELEASE}` against {release} and `{SK}` against \
         {sk}"
    );
    let heavy = r"^(?:\B\B\B\B\B\B\B\B-){200}$";
    let texts = [
        (
            "contracts/service/compatibility.md",
            include_str!("../../../../contracts/service/compatibility.md"),
            vec![
                guide.clone(),
                format!(
                    "(`{LIVE_RANGES}` on {} bytes, `^(?i-u:x*x{{150}})$` on {}, \
                     `^(?s:.*)a?x{{150}}$` on {})",
                    bound(LIVE_RANGES),
                    bound("^(?i-u:x*x{150})$"),
                    bound("^(?s:.*)a?x{150}$")
                ),
                "`max_value_bytes`: 2,293,760 divided by the weight of the pattern's compiled \
                 program"
                    .to_owned(),
            ],
        ),
        (
            "contracts/cli/v1alpha1/semantics.md",
            include_str!("../../../../contracts/cli/v1alpha1/semantics.md"),
            vec![
                guide.clone(),
                format!(
                    "such as `{heavy}`, which is matched against {} bytes",
                    bound(heavy)
                ),
            ],
        ),
        (
            "docs/local-approvals.md",
            include_str!("../../../../docs/local-approvals.md"),
            vec![guide.clone()],
        ),
        (
            "docs/running-services.md",
            include_str!("../../../../docs/running-services.md"),
            vec![guide],
        ),
        (
            "CHANGELOG.md",
            include_str!("../../../../CHANGELOG.md"),
            vec![format!("(`{DEV}`: {dev} bytes; `{RELEASE}`: {release})")],
        ),
    ];
    let mut missing = Vec::new();
    for (name, text, phrases) in &texts {
        let text = squeezed(text);
        for phrase in phrases {
            if !text.contains(phrase.as_str()) {
                missing.push(format!("{name}: {phrase}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the texts do not state what the reader computes: {missing:#?}"
    );
    // The work constant the contract names is the one the reader divides:
    // the pattern of the guide that weighs 280 keeps 2,293,760 / 280 bytes.
    assert_eq!(bound_of(&read(RELEASE)), 2_293_760 / 280);
}

/// compatibility.md § 2.1: "4,393,287 bytes (4.19 MiB) at the most for the
/// file with the highest peak found, 63 members of the costliest pattern
/// above and one member of 60 classes of `\w` nested in one another". The
/// suite prints the peak of other files and asserts only that it is under
/// 5 MiB; here the peak of this file is held to the stated figure within
/// 4,096 bytes, as the other stated figures are, and the text is held to the
/// figure and to the two patterns of the file.
///
/// Changed in fix round 3 together with the contract's text: the figure was
/// 4,251,225 bytes for 64 members of `^(?s:.*)[...]{146}$`, which this case
/// measured as 4,250,446. The security review of round 3 found the file
/// measured here, with the higher peak
/// (`security_r3::one_read_and_one_checked_write_peak_under_5_mib_for_other_shapes`).
#[test]
fn one_read_and_one_checked_write_peak_at_the_figure_the_contract_states() {
    const STATED_PEAK: isize = 4_393_287;
    let costliest = r"^[\x01\x03\x05\x07\x09\x0b\x0d\x0f\x11\x13\x15\x17\x19\x1b\x1d\x1f!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{}\x7f]{113}$";
    let nested = format!("^{}{}$", r"[\w".repeat(60), "]".repeat(60));
    assert!(nested.len() <= 256, "{} bytes", nested.len());
    let mut members: Vec<Value> = (0..63)
        .map(|n| {
            grant(
                &format!("g{n}"),
                json!([{"pointer": "/k", "pattern": costliest}]),
            )
        })
        .collect();
    members.push(grant("g63", json!([{"pointer": "/k", "pattern": nested}])));
    let file = document(json!(members));
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let input = json!({"k": "a".repeat(113)});
    let before = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(before));
    let grants = standing::parse(&file).expect("a file within the stated limits");
    let covering = covering(&grants, &input);
    let holds = LIVE.with(Cell::get) - before;
    let peak = PEAK.with(Cell::get) - before;
    drop(grants);
    // The value is 113 characters and the nested class admits one.
    assert_eq!(covering, Ok(63));
    println!("{holds} bytes held after one read and one checked write, {peak} at the most");
    assert!(
        (peak - STATED_PEAK).abs() <= 4_096,
        "one read and one checked write of 63 members of {costliest} and one of {nested} hold \
         {peak} bytes at the most; the contract states {STATED_PEAK}"
    );
    let text = squeezed(include_str!(
        "../../../../contracts/service/compatibility.md"
    ));
    let stated = format!(
        "{} bytes ({:.2} MiB) at the most for the file with the highest peak found, 63 members \
         of the costliest pattern above and one member of 60 classes of `\\w` nested in one \
         another (`^[\\w[\\w...]]$`, with `[\\w` written 60 times and `]` 60 times)",
        written(STATED_PEAK as usize),
        STATED_PEAK as f64 / (1024.0 * 1024.0)
    );
    assert!(
        text.contains(&stated),
        "the contract names another file or figure; this case measured {peak} bytes and \
         expects the text: {stated}"
    );
    assert!(
        text.contains(&format!(
            "(`{costliest}`) checked against a value of 113 times `a`"
        )),
        "the costliest pattern the contract names is not the one measured here"
    );
}
