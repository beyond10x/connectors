//! Security review of story:standing-grant-binds-a-pattern-or-a-value-set:
//! the grant file's reader, driven from the contract
//! (contracts/service/compatibility.md § 2.1, Keyed writes under a standing
//! grant) and from the public reader `local::standing::parse`, which the
//! served host and the local signed file both read a document through.
//!
//! - What one read of a file within the stated limits holds and how long it
//!   takes (the contract states the figures of "the costliest pattern
//!   found").
//! - A member the reader does not understand refuses the whole file.
//! - A pattern holds the whole value under the flags a signer can write.
#![cfg(unix)]
use crate::support::LIVE;
use connectors_host::local::standing::{self, Uncovered, Unreadable};
use serde_json::{Value, json};
use std::{cell::Cell, time::Duration};

const INSTANCE: &str = "leaf";
const OPERATION: &str = "issue.transition.run";
const LATER: i64 = 2_000_000_000_000;

/// compatibility.md § 2.1: "a file of 64 such members 3,950,582 bytes
/// (3.77 MiB)", the figure of the costliest pattern found, which the
/// contract states as a measured worst case and not as a proven ceiling.
///
/// Changed in the fix round of this review together with the contract's
/// text: the figure was 27,278,992 bytes for 64 members of `^\b\w\b$`, the
/// costliest pattern the contract named before the matcher was rebuilt from
/// the lazy DFA and the PikeVM only.
///
/// Changed in fix round 2 together with the contract's text: the figure was
/// 21,536,886 bytes for 64 members of `^\ba?x{2034}\b$`, the costliest
/// pattern the contract named before the reader matched with the PikeVM
/// alone and against no value longer than a pattern's `max_value_bytes`.
const STATED_ONE_READ: isize = 3_950_582;

/// compatibility.md § 2.1: "no more than 5 MiB at any moment", the rounded
/// bound on what one read and one checked write hold (fix round 2).
const STATED_BOUND: isize = 5 * 1024 * 1024;

/// The contract's costliest pattern: 113 times one class of 64 byte ranges,
/// the characters of odd code point from U+0001 to U+007F (fix round 2).
const COSTLIEST: &str = r"^[\x01\x03\x05\x07\x09\x0b\x0d\x0f\x11\x13\x15\x17\x19\x1b\x1d\x1f!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{}\x7f]{113}$";

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
/// `{"pointer": "/k", "pattern": pattern}`: every grant of the instance and
/// the operation is checked against a write, so one write whose `/k` is a
/// string meets every member.
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

/// One read of `file` and the check of the write `{"k": value}` against it:
/// the bytes this thread holds once the write was checked, with the grants
/// still held as the host holds them until it has taken a use, and how many
/// grants cover the write.
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

/// The control of the next test: the contract's own costliest pattern,
/// measured here as the contract measured it. (Fix round: the contract's
/// costliest pattern is now `^\ba?x{2034}\b$` on 2,034 times `x`, and the
/// band around its figure is 262,144 bytes on both sides; it was open
/// 1,278,992 bytes downwards.)
///
/// Changed in fix round 2 together with the contract's text: the costliest
/// pattern is [`COSTLIEST`] on 113 times `a`, and the band around its
/// figure is 65,536 bytes on both sides (it was 262,144, which is what a
/// matcher that holds 64 bytes a pattern and CPU adds on a host of 64 CPUs).
/// The read is also held under the contract's rounded bound.
#[test]
fn the_costliest_pattern_of_the_contract_holds_what_the_contract_states() {
    let file = file_of(COSTLIEST, 64);
    assert!(file.len() <= standing::FILE_LIMIT);
    let (holds, covering) = one_read(&file, &"a".repeat(113));
    assert_eq!(covering, 64);
    // The document itself (ids, pointers, texts) is counted here too.
    assert!(
        (STATED_ONE_READ - 65_536..=STATED_ONE_READ + 65_536).contains(&holds),
        "one read of 64 members of {COSTLIEST} holds {holds} bytes; the contract states {STATED_ONE_READ}"
    );
    assert!(holds <= STATED_BOUND, "{holds}");
    // The costliest pattern the contract named before holds less now.
    let file = file_of(r"^\b\w\b$", 64);
    let (holds, covering) = one_read(&file, "a");
    assert_eq!(covering, 64);
    assert!(
        holds <= STATED_ONE_READ,
        "one read of 64 members of `^\\b\\w\\b$` holds {holds} bytes"
    );
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

/// Invariant 6, memory. A file a signer can write within every stated limit
/// (64 KiB, 64 pattern members, each pattern at most 256 bytes and accepted
/// under the compiled-size limit) holds, in one read and one checked write,
/// no more than the contract states for one read. The pattern is 100 empty
/// capture groups before `[^\x00-\x7f]?-{1725}`: the `regex` crate sizes the
/// cache of its fallback matcher by states times capture slots, which
/// neither `size_limit` nor `dfa_size_limit` bounds. (Before the fix round
/// this read held 417,611,838 bytes against a stated 27,278,992.)
///
/// Changed in fix round 2 (the work bound): the reader matches this pattern
/// against fewer bytes than the 1,725 it needs, so the value of 1,725 dashes
/// is inside no grant (the case asserted 64 covering grants). It is still
/// measured, and so is a run of dashes of the pattern's bound, on which the
/// matcher runs, and the same pattern with 100 dashes on its value, which
/// is inside all 64. The bound on what the read holds is as it was, against
/// the figure the contract states now.
#[test]
fn one_read_of_a_file_within_the_stated_limits_holds_no_more_than_the_contract_states() {
    let groups = "()".repeat(100);
    let pattern = format!(r"^{groups}[^\x00-\x7f]?-{{1725}}$");
    assert!(pattern.len() <= 256, "{} bytes", pattern.len());
    let file = file_of(&pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let bound = longest(&file);
    assert!(bound < 1725, "the pattern is matched against {bound} bytes");
    let short = format!(r"^{groups}[^\x00-\x7f]?-{{100}}$");
    let short_file = file_of(&short, 64);
    for (file, pattern, dashes, inside) in [
        (&file, &pattern, 1725, 0),
        (&file, &pattern, bound, 0),
        (&short_file, &short, 100, 64),
    ] {
        let (holds, covering) = one_read(file, &"-".repeat(dashes));
        assert_eq!(covering, inside, "{dashes} dashes under {pattern}");
        assert!(
            holds <= STATED_ONE_READ,
            "one read of a {}-byte file with 64 pattern members of {} bytes each holds {holds} \
             bytes after one checked write of {dashes} bytes; the contract states \
             {STATED_ONE_READ} bytes for 64 members of the costliest pattern found",
            file.len(),
            pattern.len(),
        );
    }
}

/// Invariant 6, time (the bound acceptance 25 of the story names: one read
/// of 64 patterns in under a second). A file within every stated limit whose
/// patterns spend their bytes on case folding of a large Unicode class:
/// `(?i)` and 82 times `\pL` in one class. The time is this thread's CPU
/// time, so the host's load does not count, and the least of two reads.
///
/// Fix round: the reader now refuses a pattern that folds case with Unicode
/// on, so the file of that pattern is no longer within the stated limits.
/// Its read is still timed here, as the refusal it now is, under the same
/// bound. The same bound then holds for the costliest patterns the reader
/// still takes that the fix round found: large Unicode classes without a
/// case fold, written 122 times in one class, as 22 symmetric differences
/// under `|` and as 22 nested in one class (measured 196 ms, 182 ms and
/// 145 ms of CPU time per read in the test profile).
#[test]
fn one_read_of_a_file_within_the_stated_limits_takes_under_a_second() {
    let timed = |pattern: &str, taken: bool| {
        assert!(pattern.len() <= 256, "{} bytes", pattern.len());
        let file = file_of(pattern, 64);
        assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
        let read = || {
            let (cpu_before, wall) = (cpu(), std::time::Instant::now());
            let grants = standing::parse(&file);
            let spent = (cpu() - cpu_before, wall.elapsed());
            match grants {
                Ok(grants) if taken => assert_eq!(grants.grants().len(), 64),
                Err(Unreadable::Malformed) if !taken => {}
                other => panic!(
                    "{pattern:?} (taken: {taken}) reads as {:?}",
                    other.map(|grants| grants.grants().len())
                ),
            }
            spent
        };
        let first = read();
        let second = read();
        let least = first.0.min(second.0);
        assert!(
            least < Duration::from_secs(1),
            "one read of a {}-byte file with 64 pattern members of {} bytes each takes {least:?} \
             of CPU time (two reads: CPU {:?} and {:?}, wall {:?} and {:?}): {pattern}",
            file.len(),
            pattern.len(),
            first.0,
            second.0,
            first.1,
            second.1,
        );
    };
    // The file of the review: refused now, and in under a second.
    timed(&format!("^(?i)[{}]$", r"\pL".repeat(82)), false);
    // The same class without the fold is taken.
    timed(&format!("^[{}]$", r"\pL".repeat(82)), true);
    timed(&format!("^[{}]$", r"\W".repeat(122)), true);
    timed(
        &format!("^(?:{})$", vec![r"[\pL~~\PC]"; 22].join("|")),
        true,
    );
    timed(&format!("^[{}]$", r"[\w~~\pL]".repeat(22)), true);
}

/// Invariant 3: a file with a member the reader does not understand, with
/// two forms on one member, with a pattern outside its form or its bounds,
/// or with more than 64 pattern members is refused as a whole, also when an
/// exact grant the reader would admit stands before it.
#[test]
fn a_member_the_reader_does_not_understand_refuses_the_whole_file() {
    let exact = grant("exact", json!([{"pointer": "/k", "value": "DEV-12"}]));
    let admitted = |file: &[u8]| {
        standing::parse(file).map(|grants| {
            grants
                .covering(INSTANCE, OPERATION, &json!({"k": "DEV-12"}), 1_000)
                .map(|covering| covering.len())
        })
    };
    // The controls: the exact grant alone, and beside a pattern grant.
    assert_eq!(admitted(&document(json!([exact.clone()]))), Ok(Ok(1)));
    let good = grant(
        "other",
        json!([{"pointer": "/k", "pattern": "^DEV-[0-9]+$"}]),
    );
    assert_eq!(admitted(&document(json!([exact.clone(), good]))), Ok(Ok(2)));

    let long = format!("^{}$", "a".repeat(255));
    assert_eq!(long.len(), 257);
    let members = [
        json!({"pointer": "/k", "pattern": "^a$", "flags": "i"}),
        json!({"pointer": "/k", "pattern": "^a$", "value": null}),
        json!({"pointer": "/k", "pattern": "^a$", "value": "a"}),
        json!({"pointer": "/k", "pattern": "^a$", "one_of": ["a"]}),
        json!({"pointer": "/k", "value": "a", "one_of": ["a"]}),
        json!({"pointer": "/k", "pattern": ["^a$"]}),
        json!({"pointer": "/k", "pattern": 1}),
        json!({"pointer": "/k", "pattern": null}),
        json!({"pointer": "/k", "one_of": "a"}),
        json!({"pointer": "/k", "one_of": {"a": 1}}),
        json!({"pointer": "/k", "one_of": null}),
        json!({"pointer": "/k", "one_of": []}),
        json!({"pointer": "/k", "one_of": ["a", "a"]}),
        json!({"pointer": "/k", "one_of": [1.5]}),
        json!({"pointer": "/k", "one_of": [[1e2]]}),
        json!({"pointer": "/k"}),
        // A member written as a sequence of its keys in order.
        json!(["/k", null, "^a$"]),
        json!(["/k", "a", "^a$"]),
        json!(["/k", "a", null, ["a"]]),
        // Outside the form: no leading `^`, no trailing unescaped `$`.
        json!({"pointer": "/k", "pattern": "a$"}),
        json!({"pointer": "/k", "pattern": "(?m)^a$"}),
        json!({"pointer": "/k", "pattern": "^a"}),
        json!({"pointer": "/k", "pattern": "^a\\$"}),
        json!({"pointer": "/k", "pattern": "^a$\n"}),
        json!({"pointer": "/k", "pattern": ""}),
        // A final `$` that `(?x)` makes a comment.
        json!({"pointer": "/k", "pattern": "^(?x)a #$"}),
        // Does not compile; over 256 bytes; over the compiled-size limit.
        json!({"pointer": "/k", "pattern": "^(a$"}),
        json!({"pointer": "/k", "pattern": "^a)|(b$"}),
        json!({"pointer": "/k", "pattern": long}),
        json!({"pointer": "/k", "pattern": "^[a-z]{907}$"}),
        json!({"pointer": "/k", "pattern": "^\\w{2}$"}),
    ];
    for member in members {
        let file = document(json!([
            exact.clone(),
            grant("other", json!([member.clone()]))
        ]));
        assert_eq!(
            admitted(&file),
            Err(Unreadable::Malformed),
            "a file with the member {member} admits nothing"
        );
    }

    // One key twice in a member: the bytes, since a JSON value holds one.
    let twice = String::from_utf8(document(json!([
        exact.clone(),
        grant("other", json!([{"pointer": "/k", "pattern": "^a$"}]))
    ])))
    .unwrap()
    .replace(
        r#""pattern":"^a$""#,
        r#""pattern":"^a$","pattern":"^(?s:.*)$""#,
    );
    assert!(twice.contains(r#""pattern":"^a$","pattern""#), "{twice}");
    assert_eq!(admitted(twice.as_bytes()), Err(Unreadable::Malformed));

    // 64 pattern members are read, 65 are not, whichever grant holds the
    // last one, an expired grant included.
    let patterned = |id: &str, from: usize, count: usize, not_after: i64| {
        let target: Vec<Value> = (from..from + count)
            .map(|n| json!({"pointer": format!("/p{n}"), "pattern": "^a$"}))
            .collect();
        let mut one = grant(id, json!(target));
        one["not_after_unix_ms"] = json!(not_after);
        one
    };
    let four: Vec<Value> = (0..4)
        .map(|n| patterned(&format!("g{n}"), 0, 16, LATER))
        .collect();
    let mut at_the_bound = vec![exact.clone()];
    at_the_bound.extend(four.clone());
    assert_eq!(admitted(&document(json!(at_the_bound))), Ok(Ok(1)));
    for not_after in [LATER, 1] {
        let mut over = vec![exact.clone()];
        over.extend(four.clone());
        over.push(patterned("last", 0, 1, not_after));
        assert_eq!(
            admitted(&document(json!(over))),
            Err(Unreadable::Malformed),
            "65 pattern members, the last in a grant ending at {not_after}"
        );
    }
}

/// Invariant 1: a pattern holds the whole string value under the flags a
/// signer can write inside it, and under no other normal form of the value
/// or of the pointer.
#[test]
fn a_pattern_holds_the_whole_value_under_every_flag_a_signer_can_write() {
    let check = |pattern: &str, pointer: &str, input: Value| {
        let file = document(json!([grant(
            "g",
            json!([{"pointer": pointer, "pattern": pattern}])
        )]));
        standing::parse(&file)
            .unwrap_or_else(|refusal| panic!("{pattern:?} is refused: {refusal:?}"))
            .covering(INSTANCE, OPERATION, &input, 1_000)
            .map(|covering| covering.len())
    };
    let other = Err(Uncovered::OtherTarget);
    let missing = Err(Uncovered::MissingMember);

    // Multi-line mode lets `^` and `$` match at a line break; the whole
    // value still has to match.
    for pattern in ["^(?m)DEV-[0-9]+$", "^(?m:^DEV-[0-9]+$)$"] {
        assert_eq!(check(pattern, "/k", json!({"k": "DEV-12"})), Ok(1));
        for refused in [
            "DEV-12\n",
            "\nDEV-12",
            "DEV-12\nOPS-1",
            "OPS-1\nDEV-12",
            "DEV-12\r\n",
        ] {
            assert_eq!(
                check(pattern, "/k", json!({"k": refused})),
                other,
                "{pattern:?} on {refused:?}"
            );
        }
    }
    // A top-level `|` whose branches each hold one anchor.
    for (value, expected) in [
        ("a", Ok(1)),
        ("b", Ok(1)),
        ("ab", other),
        ("ax", other),
        ("xb", other),
        ("a\nb", other),
    ] {
        assert_eq!(
            check("^a|b$", "/k", json!({"k": value})),
            expected,
            "{value:?}"
        );
    }
    // CRLF mode and swapped greediness change no whole-value answer.
    assert_eq!(
        check("^(?R)DEV-[0-9]+$", "/k", json!({"k": "DEV-12\r"})),
        other
    );
    assert_eq!(
        check("^(?U)DEV-[0-9]+$", "/k", json!({"k": "DEV-12x"})),
        other
    );
    // No case fold and no look-alike the signer did not write: fullwidth
    // letters and digits, a combining mark, a zero-width joiner, a NUL.
    for refused in [
        "dev-12",
        "DEV-\u{ff11}\u{ff12}",
        "\u{ff24}EV-12",
        "DEV-12\u{301}",
        "DEV-1\u{200d}2",
        "DEV-12\u{0}",
        "DEV\u{2010}12",
        " DEV-12",
        "DEV-12 ",
    ] {
        assert_eq!(
            check("^DEV-[0-9]+$", "/k", json!({"k": refused})),
            other,
            "{refused:?}"
        );
    }
    // Only a string: the number and the nested forms of the same text.
    for refused in [
        json!(12),
        json!(["DEV-12"]),
        json!({"0": "DEV-12"}),
        Value::Null,
    ] {
        assert_eq!(
            check("^(?s:.*)$", "/k", json!({"k": refused.clone()})),
            other,
            "{refused}"
        );
    }
    // The pointer is read as written: no other spelling of an index or of a
    // name reaches the member.
    let list = json!({"list": ["x", "DEV-12"], "a/b": "DEV-12", "A": "DEV-12"});
    assert_eq!(check("^DEV-[0-9]+$", "/list/1", list.clone()), Ok(1));
    assert_eq!(check("^DEV-[0-9]+$", "/a~1b", list.clone()), Ok(1));
    for pointer in ["/list/01", "/list/+1", "/list/1.0", "/list/-", "/a", "/a/b"] {
        assert_eq!(
            check("^DEV-[0-9]+$", pointer, list.clone()),
            missing,
            "{pointer}"
        );
    }
}
