//! Adversary review of story:standing-grant-binds-a-pattern-or-a-value-set:
//! a target member of a standing grant bound to a `pattern` or a `one_of`
//! (contracts/service/compatibility.md § 2.1; contracts/cli/v1alpha1/semantics.md,
//! "Standing grants"; docs/local-approvals.md, "A grant for many targets").
//!
//! Driven from the contract texts and the story, through the public surface
//! both placements call: `standing::parse`, `Grants::covering`,
//! `standing::check_request` and the served `POST /v1alpha2/invoke`.
//!
//! Four cases are red on the tree this review covers; each says so in its
//! comment. The others hold and stay as the cases that tell the rule from the
//! two mutants named in `Rule`.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{ErrorCode, InvokeResponse},
};
use connectors_host::{
    local::standing::{self, Grants, Refused, Uncovered, Unreadable, Uses},
    server::{ServiceState, router_with_state},
};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const FORMAT: &str = "connectors.standing-grants/1";
const INSTANCE: &str = "leaf";
const OPERATION: &str = "op.write";
const LATER: i64 = 2_000_000_000_000;
const NOW: i64 = 1_000;

// ---------------------------------------------------------------------------
// The grant file as a served host reads it: bytes through `standing::parse`.
// ---------------------------------------------------------------------------

fn document(grants: Value, revoked: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"format": FORMAT, "grants": grants, "revoked": revoked})).unwrap()
}

/// A grant of `leaf` for `op.write` with `target` and, when given,
/// `input_keys`.
fn grant(id: &str, target: Value, input_keys: Option<Value>) -> Value {
    let mut grant = json!({
        "id": id, "instance": INSTANCE, "operation": OPERATION,
        "target": target, "not_after_unix_ms": LATER, "max_per_hour": 5
    });
    if let Some(input_keys) = input_keys {
        grant["input_keys"] = input_keys;
    }
    grant
}

fn read(grants: Value) -> std::result::Result<Grants, Unreadable> {
    standing::parse(&document(grants, json!([])))
}

/// The ids of the grants covering `input`.
fn covers(grants: &Grants, input: &Value) -> std::result::Result<Vec<String>, Uncovered> {
    grants
        .covering(INSTANCE, OPERATION, input, NOW)
        .map(|covering| covering.iter().map(|grant| grant.id.clone()).collect())
}

/// Whether a file of one open grant whose only member is `member` covers
/// `input`.
fn member_admits(member: &Value, input: &Value) -> bool {
    let grants = read(json!([grant("g1", json!([member]), None)]))
        .unwrap_or_else(|refusal| panic!("{member} is refused: {refusal:?}"));
    covers(&grants, input).is_ok()
}

/// What the tree answers for `value` under `pattern`: `None` when the grant
/// file that holds the pattern is refused.
fn built(pattern: &str, value: &str) -> Option<bool> {
    let member = json!({"pointer": "/k", "pattern": pattern});
    match read(json!([grant("g1", json!([member]), None)])) {
        Err(Unreadable::Malformed) => None,
        Err(other) => panic!("{pattern:?}: {other:?}"),
        Ok(grants) => Some(covers(&grants, &json!({"k": value})).is_ok()),
    }
}

/// An input as either placement reads it from the bytes it was sent.
fn sent(input: &str) -> Value {
    connectors_core::read_json(input.as_bytes())
        .unwrap_or_else(|_| panic!("{input} is not an input the reader takes"))
}

// ---------------------------------------------------------------------------
// The rule of `Pattern::new` and `Pattern::matches`, copied, and two mutants
// of it. The mutation lives here, in the test; no file of the tree is changed.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    /// `Pattern::new` and `Pattern::matches` as the tree has them.
    AsBuilt,
    /// Mutant 1: the wrapped form `\A(?:pattern)\z` must still compile, but a
    /// value is matched against the pattern as written.
    MatchesAsWritten,
    /// Mutant 2: the pattern is not compiled on its own first; only the
    /// wrapped form is.
    NoCompileOnItsOwn,
}

fn compiled(pattern: &str) -> Option<regex::Regex> {
    regex::RegexBuilder::new(pattern)
        .size_limit(64 * 1024)
        .dfa_size_limit(64 * 1024)
        .build()
        .ok()
}

fn answer(rule: Rule, pattern: &str, value: &str) -> Option<bool> {
    if pattern.len() > 256
        || !pattern.starts_with('^')
        || !pattern.ends_with('$')
        || pattern.ends_with("\\$")
    {
        return None;
    }
    let alone = compiled(pattern);
    if rule != Rule::NoCompileOnItsOwn && alone.is_none() {
        return None;
    }
    let whole = compiled(&format!(r"\A(?:{pattern})\z"))?;
    Some(match (rule, alone) {
        (Rule::MatchesAsWritten, Some(alone)) => alone.is_match(value),
        _ => whole.is_match(value),
    })
}

/// The shared vectors of the story (acceptance 26), as the two crates read
/// them.
fn shared_vectors() -> Vec<(String, String, Option<bool>)> {
    let shared: Value =
        serde_json::from_str(include_str!("../fixtures/whole_value_patterns.json")).unwrap();
    shared["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|vector| {
            (
                vector["pattern"].as_str().unwrap().to_owned(),
                vector["value"].as_str().unwrap().to_owned(),
                vector["matches"].as_bool(),
            )
        })
        .collect()
}

/// Every pattern and value the story's own tests in `local::standing` and in
/// `http_standing_grant_adversary.rs` put to the rule, with what they assert.
fn suite_cases() -> Vec<(String, String, Option<bool>)> {
    let dev = "^DEV-[0-9]+$";
    let either = "^a$|^b$";
    let review = "^Release review of (?s:.*)$";
    let mut cases = vec![
        (dev, "DEV-12", Some(true)),
        (dev, "DEV-12x", Some(false)),
        (dev, "xDEV-12", Some(false)),
        (dev, "dev-12", Some(false)),
        (dev, "DEV-12\n", Some(false)),
        (dev, "DEV-", Some(false)),
        (dev, "", Some(false)),
        (dev, "OPS-12", Some(false)),
        (dev, "DEV-0", Some(true)),
        (dev, "DEV-499", Some(true)),
        (dev, "DEV-99999", Some(true)),
        (either, "a", Some(true)),
        (either, "b", Some(true)),
        (either, "ab", Some(false)),
        (either, "a\nb", Some(false)),
        (either, "ba", Some(false)),
        (either, "", Some(false)),
        (review, "Release review of 2.46.8", Some(true)),
        (review, "Release review of 2.46.8\nsecond line", Some(true)),
        (review, "Note: Release review of 2.46.8", Some(false)),
        ("^x$", "x", Some(true)),
        ("^x$", "y", Some(false)),
        ("^a$", "a", Some(true)),
        ("^(a)$", "a", Some(true)),
        ("^(a$", "a", None),
        ("DEV-[0-9]+$", "DEV-12", None),
        ("^DEV-[0-9]+", "DEV-12", None),
        (r"^DEV-[0-9]+\$", "DEV-12", None),
        ("", "", None),
    ]
    .into_iter()
    .map(|(pattern, value, matches)| (pattern.to_owned(), value.to_owned(), matches))
    .collect::<Vec<_>>();
    cases.push((
        format!("^{}$", "a".repeat(254)),
        "a".repeat(254),
        Some(true),
    ));
    cases.push((format!("^{}$", "a".repeat(255)), "a".repeat(255), None));
    cases
}

/// The cases of `cases` on which `rule` answers something else than the case
/// expects: the cases that would turn a suite red under the mutant.
fn killers(rule: Rule, cases: &[(String, String, Option<bool>)]) -> Vec<String> {
    cases
        .iter()
        .filter(|(pattern, value, matches)| answer(rule, pattern, value) != *matches)
        .map(|(pattern, value, _)| format!("{pattern:?} on {value:?}"))
        .collect()
}

/// The control of the two cases below: the copy above is the rule as built.
/// It answers every shared vector and every case of the suite as expected,
/// and the tree answers the same.
#[test]
fn the_copied_rule_is_the_rule_as_built() {
    let mut cases = shared_vectors();
    assert!(cases.len() >= 30, "{}", cases.len());
    cases.extend(suite_cases());
    assert_eq!(killers(Rule::AsBuilt, &cases), Vec::<String>::new());
    for (pattern, value, matches) in &cases {
        assert_eq!(built(pattern, value), *matches, "{pattern:?} on {value:?}");
    }
}

/// RED on the tree under review. Invariant 1 rests on the wrap
/// `\A(?:pattern)\z` (`Pattern::new`: "The wrap is what holds the whole
/// value"). No shared vector and no case of the story's tests answers
/// differently when a value is matched against the pattern as written
/// instead: `^a$|^b$` anchors both branches and `^(?m:^a$)$` scopes the flag,
/// so neither needs the wrap. The host's copy of the rule can lose the wrap
/// with every test green. Fix: vectors such as `^a|b$` on `ab` (false) and
/// `^(?m)a$` on `a\nb` (false) in `fixtures/whole_value_patterns.json`.
#[test]
fn a_shared_vector_or_a_suite_case_needs_the_wrap_to_hold() {
    let mut cases = shared_vectors();
    cases.extend(suite_cases());
    let killers = killers(Rule::MatchesAsWritten, &cases);
    assert!(
        !killers.is_empty(),
        "none of {} cases tells `\\A(?:pattern)\\z` from the pattern as written",
        cases.len()
    );
}

/// RED on the tree under review. `Pattern::new` compiles the pattern on its
/// own first, "so the wrap's `(?:` and `)` enclose all of it". The only
/// vector with unbalanced groups, `^(a$`, fails wrapped too, so no case tells
/// whether that first compile is there. Fix: the vector `^a)|(?:b$` (null).
#[test]
fn a_shared_vector_or_a_suite_case_needs_the_compile_on_its_own() {
    let mut cases = shared_vectors();
    cases.extend(suite_cases());
    let killers = killers(Rule::NoCompileOnItsOwn, &cases);
    assert!(
        !killers.is_empty(),
        "none of {} cases tells whether the pattern is compiled on its own",
        cases.len()
    );
}

/// Holds. The cases the two above ask for: the tree answers them as the
/// contract says, and each mutant answers at least one of them wrongly.
#[test]
fn one_anchor_per_branch_and_a_multi_line_flag_still_hold_the_whole_value() {
    let cases: Vec<(String, String, Option<bool>)> = [
        ("^DEV-[0-9]+|x$", "DEV-12", Some(true)),
        ("^DEV-[0-9]+|x$", "x", Some(true)),
        ("^DEV-[0-9]+|x$", "DEV-12/../admin", Some(false)),
        ("^DEV-[0-9]+|x$", "OPS-1x", Some(false)),
        ("^a|b$", "ab", Some(false)),
        ("^(?m)DEV-[0-9]+$", "DEV-12", Some(true)),
        ("^(?m)DEV-[0-9]+$", "DEV-12\nOPS-1", Some(false)),
        ("^(?m)DEV-[0-9]+$", "DEV-12\n", Some(false)),
        ("^(?m)a$", "a\nb", Some(false)),
        // Groups that balance only inside the wrap: `\A(?:^a)|(?:b$)\z`.
        ("^a)|(?:b$", "a/../x", None),
        ("^a)|(?:b$", "xb", None),
        ("^a)$", "a", None),
    ]
    .into_iter()
    .map(|(pattern, value, matches)| (pattern.to_owned(), value.to_owned(), matches))
    .collect();
    for (pattern, value, matches) in &cases {
        assert_eq!(built(pattern, value), *matches, "{pattern:?} on {value:?}");
    }
    assert_eq!(killers(Rule::AsBuilt, &cases), Vec::<String>::new());
    assert!(!killers(Rule::MatchesAsWritten, &cases).is_empty());
    assert!(!killers(Rule::NoCompileOnItsOwn, &cases).is_empty());
    // What the compile on its own keeps out: the wrapped form of this text
    // is two alternatives with one anchor each.
    let open = regex::Regex::new(r"\A(?:^a)|(?:b$)\z").unwrap();
    assert!(open.is_match("a/../x") && open.is_match("xb"));
}

// ---------------------------------------------------------------------------
// Invariant 1: what a pattern admits.
// ---------------------------------------------------------------------------

/// RED on the tree under review. Invariant 1: no match through "a Unicode
/// look-alike or a case fold the signer did not ask for". The contract texts
/// name `\d`, `\w`, `\s` and `\b` as Unicode-aware and say matching is
/// case-sensitive; the shared vectors hold `^(?i)dev$` as the way to ask for
/// a case fold. A signer who asks for it gets Unicode simple case folding:
/// `s` also admits U+017F (long s) and `k` U+212A (Kelvin sign).
///
/// Changed in the review-fix round, which decided to refuse the pattern: the
/// reader takes no pattern that turns case-insensitive matching on while
/// Unicode matching is on. The two patterns of this case are therefore
/// refused, which the first loop asserts (it was the control that they admit
/// `SK-12`), and the control and the look-alikes are asserted unchanged on
/// the spelling the contract texts now name for a case fold, `(?i-u)`. The
/// vector of the shared file is `^(?i-u)dev$`.
#[test]
fn a_case_insensitive_pattern_admits_no_unicode_look_alike() {
    for pattern in ["^(?i)sk-[0-9]+$", "^(?i)[a-z]+-[0-9]+$"] {
        for value in [
            "SK-12",
            "sk-12",
            "Sk-12",
            "\u{17f}k-12",
            "s\u{212a}-12",
            "\u{17f}\u{212a}-12",
        ] {
            assert_eq!(built(pattern, value), None, "{pattern} is taken");
        }
    }
    for pattern in ["^(?i-u)sk-[0-9]+$", "^(?i-u)[a-z]+-[0-9]+$"] {
        let member = json!({"pointer": "/k", "pattern": pattern});
        // The control: the case fold the signer asked for.
        for value in ["SK-12", "sk-12", "Sk-12"] {
            assert!(member_admits(&member, &json!({"k": value})), "{value:?}");
        }
        let admitted: Vec<String> = ["\u{17f}k-12", "s\u{212a}-12", "\u{17f}\u{212a}-12"]
            .into_iter()
            .filter(|value| member_admits(&member, &json!({"k": value})))
            .map(|value| value.escape_unicode().to_string())
            .collect();
        assert_eq!(
            admitted,
            Vec::<String>::new(),
            "{pattern} admits values that hold no ASCII s or k"
        );
    }
}

/// Holds, and is the fix a contract sentence can name: with the Unicode flag
/// off the fold is ASCII only.
#[test]
fn an_ascii_case_fold_is_written_with_the_unicode_flag_off() {
    let member = json!({"pointer": "/k", "pattern": "^(?i-u:sk)-[0-9]+$"});
    for value in ["SK-12", "sk-12", "Sk-12"] {
        assert!(member_admits(&member, &json!({"k": value})), "{value:?}");
    }
    for value in ["\u{17f}k-12", "s\u{212a}-12", "\u{17f}\u{212a}-12"] {
        assert!(
            !member_admits(&member, &json!({"k": value})),
            "{}",
            value.escape_unicode()
        );
    }
}

/// RED on the tree under review. docs/local-approvals.md, "A grant for many
/// targets": "`.` does not match a line break; `(?s:.*)` does", and invariant
/// 1: "no match across a newline that the signer did not write". In the
/// `regex` crate `.` leaves out U+000A only, so a pattern written to hold one
/// line admits a carriage return and the other line breaks of Unicode.
///
/// Changed in the review-fix round: the sentence was false and the rule
/// stays, so the text now reads "`.` matches every character but U+000A, so
/// it also matches a carriage return and the other line breaks of Unicode",
/// and this case asserts that sentence: U+000A is the one line break `.`
/// does not cross (it asserted that `.` crosses none of the six below).
/// Renamed in fix round 2 from `a_dot_crosses_no_line_break`, which named
/// the sentence that was false.
#[test]
fn a_dot_stops_at_a_line_feed_and_crosses_every_other_line_break() {
    let member = json!({"pointer": "/k", "pattern": "^Release review of .*$"});
    let text = |between: &str| json!({"k": format!("Release review of 1{between}2")});
    // The control: one line is admitted, U+000A and CR LF are not crossed.
    assert!(member_admits(&member, &text("")));
    assert!(!member_admits(&member, &text("\n")));
    assert!(!member_admits(&member, &text("\r\n")));
    let others = ["\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}"];
    let crossed: Vec<&str> = others
        .into_iter()
        .filter(|between| member_admits(&member, &text(between)))
        .collect();
    assert_eq!(
        crossed, others,
        "`.` matches every character but U+000A, these line breaks included"
    );
    // `(?s:.*)` is what crosses U+000A too.
    let any = json!({"pointer": "/k", "pattern": "^Release review of (?s:.*)$"});
    for between in others.into_iter().chain(["\n", "\r\n"]) {
        assert!(member_admits(&any, &text(between)), "{between:?}");
    }
}

/// Holds. The consumer's pattern admits no other normal form of a ticket key:
/// look-alikes, invisible characters, line ends, other JSON types, and the
/// escapes of the bytes a caller sends.
#[test]
fn the_ticket_pattern_admits_one_spelling_of_a_key() {
    let member = json!({"pointer": "/k", "pattern": "^DEV-[0-9]+$"});
    assert!(member_admits(&member, &json!({"k": "DEV-12"})));
    assert!(member_admits(&member, &sent(r#"{"k":"DEV-12"}"#)));
    for value in [
        "DEV-\u{ff11}\u{ff12}",
        "DEV-\u{661}\u{662}",
        "DEV-12\u{200b}",
        "\u{feff}DEV-12",
        "DEV-12\0",
        "DEV-12\r",
        "DEV-12\r\n",
        "DEV-12\n",
        "\nDEV-12",
        "DEV-12 ",
        " DEV-12",
        "D\u{415}V-12",
        "DEV\u{2010}12",
        "DEV-12\u{301}",
        "\u{ff24}\u{ff25}\u{ff36}-12",
        "dev-12",
        "DEV-12\nDEV-13",
        "DEV-12/../DEV-13",
    ] {
        assert!(
            !member_admits(&member, &json!({"k": value})),
            "{}",
            value.escape_unicode()
        );
    }
    for input in [
        r#"{"k":"DEV-12\u000a"}"#,
        r#"{"k":"DEV-12\n"}"#,
        r#"{"k":12}"#,
        r#"{"k":["DEV-12"]}"#,
        r#"{"k":{"0":"DEV-12"}}"#,
        r#"{"k":null}"#,
        r#"{"k":true}"#,
        r#"{"K":"DEV-12"}"#,
        r#"{"k ":"DEV-12"}"#,
        r#"{"j":{"k":"DEV-12"}}"#,
    ] {
        assert!(!member_admits(&member, &sent(input)), "{input}");
    }
    // One member name twice is no input at all.
    assert!(connectors_core::read_json::<Value>(br#"{"k":"DEV-12","k":"OPS-1"}"#).is_err());
}

/// Holds. A pointer has one spelling: an escaped name, an array index.
#[test]
fn a_pattern_member_is_found_through_one_spelling_of_its_pointer() {
    let dev = "^DEV-[0-9]+$";
    for (pointer, admitted, refused) in [
        (
            "/a~1b",
            vec![r#"{"a/b":"DEV-1"}"#],
            vec![r#"{"a":{"b":"DEV-1"}}"#, r#"{"a~1b":"DEV-1"}"#],
        ),
        (
            "/m~0n",
            vec![r#"{"m~n":"DEV-1"}"#],
            vec![r#"{"m~0n":"DEV-1"}"#, r#"{"mn":"DEV-1"}"#],
        ),
        (
            "/list/0",
            vec![r#"{"list":["DEV-1","OPS-1"]}"#, r#"{"list":{"0":"DEV-1"}}"#],
            vec![
                r#"{"list":["OPS-1","DEV-1"]}"#,
                r#"{"list":[]}"#,
                r#"{"list":"DEV-1"}"#,
            ],
        ),
        ("/list/00", vec![], vec![r#"{"list":["DEV-1"]}"#]),
        ("/list/+0", vec![], vec![r#"{"list":["DEV-1"]}"#]),
        ("/list/-", vec![], vec![r#"{"list":["DEV-1"]}"#]),
        ("/list/0x0", vec![], vec![r#"{"list":["DEV-1"]}"#]),
        ("/list/1e0", vec![], vec![r#"{"list":["OPS-1","DEV-1"]}"#]),
        ("/", vec![r#"{"":"DEV-1"}"#], vec![r#"{"k":"DEV-1"}"#]),
    ] {
        let member = json!({"pointer": pointer, "pattern": dev});
        for input in admitted {
            assert!(member_admits(&member, &sent(input)), "{pointer} {input}");
        }
        for input in refused {
            assert!(!member_admits(&member, &sent(input)), "{pointer} {input}");
        }
    }
}

/// Holds. Flags and comments inside a pattern: each text is refused, or holds
/// the whole value. `None` in the second column: either is right.
#[test]
fn a_flag_or_a_comment_inside_a_pattern_opens_nothing() {
    // A pattern, whether it is taken, and values with their answers.
    type Case = (&'static str, Option<bool>, Vec<(&'static str, bool)>);
    let cases: Vec<Case> = vec![
        ("^(?x)a#$", Some(false), vec![]),
        ("^(?x)a\n#$", Some(false), vec![]),
        ("^a|(?x)#$", Some(false), vec![]),
        (
            "^(?x: a # c\n)$",
            Some(true),
            vec![
                ("a", true),
                ("a # c\n", false),
                ("ab", false),
                (" a ", false),
            ],
        ),
        (
            "^(?m)a$",
            Some(true),
            vec![
                ("a", true),
                ("a\nb", false),
                ("a\n", false),
                ("b\na", false),
            ],
        ),
        (
            "^(?m:a$\n^b)$",
            Some(true),
            vec![
                ("a\nb", true),
                ("a", false),
                ("a\nb\nc", false),
                ("a\nb\n", false),
            ],
        ),
        (
            "^a|b$",
            Some(true),
            vec![
                ("a", true),
                ("b", true),
                ("ab", false),
                ("ax", false),
                ("xb", false),
                ("", false),
            ],
        ),
        ("^a$$", Some(true), vec![("a", true), ("a\n", false)]),
        ("^^a$", Some(true), vec![("a", true), ("\na", false)]),
        ("^(?s).*$", Some(true), vec![("x\ny", true)]),
        ("^.*$", Some(true), vec![("ab", true), ("a\nb", false)]),
        (
            "^(?R)a$",
            None,
            vec![
                ("a", true),
                ("a\r", false),
                ("a\r\n", false),
                ("a\n", false),
            ],
        ),
        ("^(?U)a+$", Some(true), vec![("aaa", true), ("aaab", false)]),
        (
            "^(?-u:[a-z])+$",
            None,
            vec![("abc", true), ("\u{e9}", false)],
        ),
        ("^a\\z$", Some(true), vec![("a", true), ("a\n", false)]),
        (
            "^|$",
            Some(true),
            vec![("", true), ("a", false), ("\n", false)],
        ),
        (
            "^(?m)$",
            Some(true),
            vec![("", true), ("\n", false), ("a", false)],
        ),
        ("^a$|", Some(false), vec![]),
        ("^a)$", Some(false), vec![]),
        ("^a)|(?:b$", Some(false), vec![]),
        (" ^a$", Some(false), vec![]),
        ("^a$ ", Some(false), vec![]),
        ("^a$\n", Some(false), vec![]),
    ];
    for (pattern, taken, values) in cases {
        let accepted = built(pattern, "").is_some();
        if let Some(taken) = taken {
            assert_eq!(accepted, taken, "{pattern:?} taken");
        }
        if accepted {
            for (value, matches) in values {
                assert_eq!(
                    built(pattern, value),
                    Some(matches),
                    "{pattern:?} on {value:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: `one_of` under the equality of `value`.
// ---------------------------------------------------------------------------

/// A grant file, written as text so a number keeps the digits it is given.
fn text_file(grants: &[String]) -> Vec<u8> {
    format!(
        r#"{{"format":"connectors.standing-grants/1","grants":[{}],"revoked":[]}}"#,
        grants.join(",")
    )
    .into_bytes()
}

/// A grant of `leaf` for `op.write` whose target is the member texts
/// `members`, joined by commas.
fn text_grant(id: &str, members: &str) -> String {
    format!(
        r#"{{"id":"{id}","instance":"leaf","operation":"op.write","target":[{members}],"not_after_unix_ms":2000000000000,"max_per_hour":5}}"#
    )
}

/// Holds. For every value a grant may bind, `{"one_of": [b]}` admits exactly
/// what `{"value": b}` admits, over inputs read from the bytes a caller
/// sends; a number neither form may bind is refused by both.
#[test]
fn a_set_of_one_value_admits_what_the_exact_member_admits() {
    let bindable = [
        "10000",
        r#""10000""#,
        "0",
        "1",
        "-1",
        r#""3""#,
        "null",
        "true",
        "false",
        r#""""#,
        r#""null""#,
        "[]",
        "{}",
        "[10000]",
        r#"{"a":[1,{"b":2}]}"#,
        r#"{"a":[1,{"b":2}],"c":1}"#,
        "18446744073709551615",
        "-9223372036854775808",
        r#""é""#,
        r#""é""#,
        r#""A""#,
        r#""a""#,
    ];
    let unbindable = [
        "1e4",
        "10000.0",
        "1.0E4",
        "-0",
        "0.0",
        "1.5",
        "18446744073709551616",
        "-9223372036854775809",
        r#"{"a":[1,{"b":2.0}]}"#,
    ];
    let exact = |bound: &str| {
        standing::parse(&text_file(&[text_grant(
            "g1",
            &format!(r#"{{"pointer":"/k","value":{bound}}}"#),
        )]))
    };
    let set = |bound: &str| {
        standing::parse(&text_file(&[text_grant(
            "g1",
            &format!(r#"{{"pointer":"/k","one_of":[{bound}]}}"#),
        )]))
    };
    for bound in unbindable {
        assert_eq!(exact(bound).err(), Some(Unreadable::Malformed), "{bound}");
        assert_eq!(set(bound).err(), Some(Unreadable::Malformed), "{bound}");
    }
    for (at, bound) in bindable.iter().enumerate() {
        let exact = exact(bound).unwrap_or_else(|refusal| panic!("{bound}: {refusal:?}"));
        let set = set(bound).unwrap_or_else(|refusal| panic!("{bound}: {refusal:?}"));
        for (other, value) in bindable.iter().chain(&unbindable).enumerate() {
            let input = sent(&format!(r#"{{"k":{value}}}"#));
            let by_value = covers(&exact, &input).is_ok();
            let by_set = covers(&set, &input).is_ok();
            assert_eq!(by_set, by_value, "{bound} bound, {value} sent");
            assert_eq!(by_set, at == other, "{bound} bound, {value} sent");
        }
        // No member at the pointer is no value of the set.
        assert_eq!(covers(&set, &sent("{}")), Err(Uncovered::MissingMember));
    }
    // The members of an object in another order: one equality for both forms.
    let ordered = r#"{"a":1,"b":2}"#;
    let input = sent(r#"{"k":{"b":2,"a":1}}"#);
    assert_eq!(
        covers(&set(ordered).unwrap(), &input).is_ok(),
        covers(&exact(ordered).unwrap(), &input).is_ok()
    );
    // A set of two admits those two of the pool and no third.
    let two = standing::parse(&text_file(&[text_grant(
        "g1",
        r#"{"pointer":"/k","one_of":[10000,"3"]}"#,
    )]))
    .unwrap();
    for (at, value) in bindable.iter().chain(&unbindable).enumerate() {
        let input = sent(&format!(r#"{{"k":{value}}}"#));
        assert_eq!(covers(&two, &input).is_ok(), at == 0 || at == 5, "{value}");
    }
}

// ---------------------------------------------------------------------------
// Invariants 3 and 7: a file is refused as a whole, and the local request
// check refuses the member the served reader refuses.
// ---------------------------------------------------------------------------

/// Member texts no reader of this release takes.
const REFUSED_MEMBERS: &[&str] = &[
    // One key twice.
    r#"{"pointer":"/k","pattern":"^a$","pattern":"^(?s:.*)$"}"#,
    r#"{"pointer":"/k","value":"a","value":"b"}"#,
    r#"{"pointer":"/k","pointer":"/j","value":"a"}"#,
    r#"{"pointer":"/k","one_of":["a"],"one_of":["b"]}"#,
    // A key the reader does not know.
    r#"{"pointer":"/k","pattern":"^a$","flags":"i"}"#,
    r#"{"pointer":"/k","pattern":"^a$","case_insensitive":true}"#,
    r#"{"pointer":"/k","regex":"^a$"}"#,
    r#"{"pointer":"/k","any_of":["a"]}"#,
    r#"{"pointer":"/k","one_of":["a"],"none_of":["b"]}"#,
    r#"{"pointer":"/k","Pattern":"^a$"}"#,
    r#"{"pointer":"/k","pattern ":"^a$"}"#,
    r#"{"pointer":"/k","value":"a","prefix":"a"}"#,
    // A key of another type.
    r#"{"pointer":"/k","pattern":["^a$"]}"#,
    r#"{"pointer":"/k","pattern":{"regex":"^a$"}}"#,
    r#"{"pointer":"/k","pattern":true}"#,
    r#"{"pointer":"/k","one_of":{"0":"a"}}"#,
    r#"{"pointer":"/k","one_of":"a"}"#,
    // Two forms, or none.
    r#"{"pointer":"/k","value":"a","pattern":"^a$"}"#,
    r#"{"pointer":"/k","value":null,"one_of":["a"]}"#,
    r#"{"pointer":"/k","pattern":"^a$","one_of":["a"]}"#,
    r#"{"pointer":"/k"}"#,
    r#"{"pointer":"/k","pattern":null}"#,
    r#"{"pointer":"/k","one_of":null}"#,
    // The members of a target member as a list, with a second or third form.
    r#"["/k",null,"^(?s:.*)$"]"#,
    r#"["/k","a",null]"#,
    r#"["/k","a","^a$",["a"]]"#,
    r#"["/k"]"#,
    // Outside the form or the bounds.
    r#"{"pointer":"/k","pattern":" ^a$"}"#,
    r#"{"pointer":"/k","pattern":"^a$ "}"#,
    r#"{"pointer":"/k","pattern":"^a$\n"}"#,
    r#"{"pointer":"/k","pattern":"^a)|(?:b$"}"#,
    r#"{"pointer":"/k","pattern":"^[a-z]{907}$"}"#,
    r#"{"pointer":"/k","one_of":[]}"#,
    r#"{"pointer":"/k","one_of":["a","a"]}"#,
    r#"{"pointer":"/k","one_of":[1.5]}"#,
    r#"{"pointer":"/k","one_of":[1e2]}"#,
    r#"{"pointer":"/k","one_of":[-0]}"#,
    r#"{"pointer":"/k","one_of":[18446744073709551616]}"#,
    r#"{"pointer":"/k","one_of":[{"a":1},{"a":1}]}"#,
];

/// Member texts every reader of this release takes.
const TAKEN_MEMBERS: &[&str] = &[
    r#"{"pointer":"/k","value":"a"}"#,
    r#"{"pointer":"/k","value":null}"#,
    r#"{"pointer":"/k","pattern":"^DEV-[0-9]+$"}"#,
    r#"{"pointer":"/k","pattern":"^a|b$"}"#,
    r#"{"pointer":"/k","pattern":"^[a-z]{906}$"}"#,
    r#"{"pointer":"/k","one_of":["a"]}"#,
    r#"{"pointer":"/k","one_of":[null,1,"1",{"a":1},{"a":2}]}"#,
];

/// The exact-value grant every file below also holds, and the write it
/// covers.
fn widgets_grant() -> String {
    text_grant(
        "g-widgets",
        r#"{"pointer":"/owner","value":"acme"},{"pointer":"/repo","value":"widgets"}"#,
    )
}

/// Holds. One member a reader does not take makes the whole file unreadable:
/// the exact-value grant beside it admits nothing.
#[test]
fn one_member_outside_the_three_forms_refuses_the_whole_file() {
    let input = sent(r#"{"owner":"acme","repo":"widgets"}"#);
    for member in TAKEN_MEMBERS {
        let file = standing::parse(&text_file(&[
            widgets_grant(),
            text_grant("g-other", member),
        ]))
        .unwrap_or_else(|refusal| panic!("{member}: {refusal:?}"));
        assert_eq!(
            covers(&file, &input),
            Ok(vec!["g-widgets".to_owned()]),
            "{member}"
        );
    }
    for member in REFUSED_MEMBERS {
        assert_eq!(
            standing::parse(&text_file(&[
                widgets_grant(),
                text_grant("g-other", member)
            ]))
            .err(),
            Some(Unreadable::Malformed),
            "{member}"
        );
        // Also as the first grant, and as a grant of another instance.
        assert_eq!(
            standing::parse(&text_file(&[
                text_grant("g-other", member),
                widgets_grant()
            ]))
            .err(),
            Some(Unreadable::Malformed),
            "{member}"
        );
        let elsewhere = text_grant("g-other", member)
            .replace(r#""instance":"leaf""#, r#""instance":"another""#);
        assert_eq!(
            standing::parse(&text_file(&[widgets_grant(), elsewhere])).err(),
            Some(Unreadable::Malformed),
            "{member}"
        );
    }
}

/// Holds. The 65th pattern member refuses the file wherever it stands: in an
/// expired grant, in a revoked one, in a grant of another instance.
#[test]
fn the_65th_pattern_member_refuses_the_file_wherever_it_stands() {
    let patterns = |id: &str, count: usize| -> Value {
        let target: Vec<Value> = (0..count)
            .map(|n| json!({"pointer": format!("/k{n}"), "pattern": "^a$"}))
            .collect();
        grant(id, json!(target), None)
    };
    let widgets = || {
        grant(
            "g-widgets",
            json!([{"pointer": "/owner", "value": "acme"}]),
            None,
        )
    };
    let full = || {
        vec![
            widgets(),
            patterns("g-1", 16),
            patterns("g-2", 16),
            patterns("g-3", 16),
            patterns("g-4", 16),
        ]
    };
    let input = json!({"owner": "acme"});
    let at_the_bound = standing::parse(&document(json!(full()), json!([]))).unwrap();
    assert_eq!(
        covers(&at_the_bound, &input),
        Ok(vec!["g-widgets".to_owned()])
    );

    let mut expired = patterns("g-5", 1);
    expired["not_after_unix_ms"] = json!(1);
    let mut elsewhere = patterns("g-5", 1);
    elsewhere["instance"] = json!("another");
    let mut other_operation = patterns("g-5", 1);
    other_operation["operation"] = json!("op.read");
    for (case, more, revoked) in [
        ("in an expired grant", expired, json!([])),
        ("in a grant of another instance", elsewhere, json!([])),
        (
            "in a grant of another operation",
            other_operation,
            json!([]),
        ),
        ("in a revoked grant", patterns("g-5", 1), json!(["g-5"])),
    ] {
        let mut grants = full();
        grants.push(more);
        assert_eq!(
            standing::parse(&document(json!(grants), revoked)).err(),
            Some(Unreadable::Malformed),
            "{case}"
        );
    }
    // A set or an exact member beside 64 patterns is no 65th pattern.
    let mut grants = full();
    grants.push(grant(
        "g-5",
        json!([{"pointer": "/k", "one_of": ["^a$"]}, {"pointer": "/j", "value": "^a$"}]),
        None,
    ));
    assert!(standing::parse(&document(json!(grants), json!([]))).is_ok());
}

/// Holds. Invariant 7 at the document: `approvals grant-standing` takes the
/// request with a member exactly when the served reader takes a file whose
/// grant holds that member.
#[test]
fn the_local_request_check_and_the_served_reader_take_the_same_members() {
    let request = |member: &str| {
        format!(
            r#"{{"operation":"op.write","target":[{member}],"not_after_unix_ms":2000000000000,"max_per_hour":5}}"#
        )
    };
    for member in TAKEN_MEMBERS {
        assert_eq!(
            standing::check_request(request(member).as_bytes(), INSTANCE),
            Ok(()),
            "{member}"
        );
        assert!(
            standing::parse(&text_file(&[text_grant("g1", member)])).is_ok(),
            "{member}"
        );
    }
    for member in REFUSED_MEMBERS {
        assert_eq!(
            standing::check_request(request(member).as_bytes(), INSTANCE),
            Err(Refused::InvalidInput),
            "{member}"
        );
        assert_eq!(
            standing::parse(&text_file(&[text_grant("g1", member)])).err(),
            Some(Unreadable::Malformed),
            "{member}"
        );
    }
}

// ---------------------------------------------------------------------------
// The array-index relaxation "opens no input".
// ---------------------------------------------------------------------------

/// A fixed sequence of numbers below `bound`.
struct Sequence(u64);
impl Sequence {
    fn below(&mut self, bound: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as u32) % bound
    }
}

/// An element of a list under `/a/list`.
fn element(sequence: &mut Sequence) -> Value {
    match sequence.below(6) {
        0 => json!({"text": "x"}),
        1 => json!({"text": "y"}),
        2 => json!({"text": "x", "more": [1, {"deep": true}]}),
        3 => json!({"text": 1}),
        4 => json!("x"),
        _ => json!({"other": "x"}),
    }
}

/// A value at `/a/list`.
fn list(sequence: &mut Sequence) -> Value {
    match sequence.below(8) {
        0 => json!([]),
        1 => json!({"0": element(sequence)}),
        2 => json!({"0": element(sequence), "1": element(sequence)}),
        3 => json!("x"),
        _ => {
            let count = 1 + sequence.below(3);
            Value::Array((0..count).map(|_| element(sequence)).collect())
        }
    }
}

/// Holds. The three contract texts: the relaxation "opens no input: the
/// listed array is admitted as without the target member, and the member
/// refuses an input whose value there it does not admit". Over 4,000 inputs
/// of a fixed sequence, the grant with the member below the listed array
/// covers exactly the inputs the grant without it covers whose
/// `/a/list/0/text` the member admits.
#[test]
fn a_member_below_a_listed_array_only_refuses() {
    let channel = json!({"pointer": "/body/channel", "value": "C1"});
    let with = read(json!([grant(
        "with",
        json!([channel.clone(), {"pointer": "/a/list/0/text", "pattern": "^x$"}]),
        Some(json!(["/a/list"])),
    )]))
    .unwrap();
    let without = read(json!([grant(
        "without",
        json!([channel]),
        Some(json!(["/a/list"])),
    )]))
    .unwrap();
    let mut sequence = Sequence(0x5eed);
    let (mut covered, mut refused_by_the_member, mut outside) = (0, 0, 0);
    for _ in 0..4_000 {
        let mut input = json!({"body": {"channel": "C1"}, "a": {"list": list(&mut sequence)}});
        match sequence.below(10) {
            0 => input["a"]["else"] = json!(1),
            1 => input["extra"] = json!({"list": [{"text": "x"}]}),
            2 => input["body"]["channel"] = json!("C2"),
            3 => drop(input["a"].as_object_mut().unwrap().remove("list")),
            _ => {}
        }
        let by_the_member = input.pointer("/a/list/0/text") == Some(&json!("x"));
        let open = covers(&without, &input).is_ok();
        let bound = covers(&with, &input).is_ok();
        assert_eq!(bound, open && by_the_member, "{input}");
        match (open, bound) {
            (true, true) => covered += 1,
            (true, false) => refused_by_the_member += 1,
            _ => outside += 1,
        }
    }
    assert!(
        covered > 200 && refused_by_the_member > 200 && outside > 200,
        "{covered} covered, {refused_by_the_member} refused by the member, {outside} outside"
    );
}

// ---------------------------------------------------------------------------
// Invariants 4 and 7 through the served host.
// ---------------------------------------------------------------------------

const TOKEN: &str = "service-token";

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

struct Fixture {
    requests: Arc<AtomicUsize>,
}
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: INSTANCE.into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![Operation {
                id: "issue.transition.run".into(),
                description: "issue.transition.run fixture".into(),
                contract: "operations/v1alpha1".into(),
                profile: "mutation".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
            }],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let number = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(json!({"operation": operation, "number": number, "input": input}))
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    requests: Arc<AtomicUsize>,
    _root: tempfile::TempDir,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Host {
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

/// A host with standing grants read from `grants` (a path under `root`).
async fn start(root: tempfile::TempDir, grants: PathBuf) -> Host {
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        requests: requests.clone(),
    });
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, "fixture", "rev-1")
            .unwrap()
            .with_standing_grants(Some(grants))
            .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        endpoint: format!("http://{address}/v1alpha2/invoke"),
        task,
        requests,
        _root: root,
    }
}

fn put(path: &Path, document: &Value) {
    let staged = path.with_extension("new");
    std::fs::write(&staged, serde_json::to_vec(document).unwrap()).unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::rename(&staged, path).unwrap();
}

/// The transition grant of the story's consumer, as an operator mounts it,
/// with the cap `max_per_hour` and the expiry `not_after_unix_ms`.
fn transition_file(max_per_hour: u32, not_after_unix_ms: i64, revoked: Value) -> Value {
    json!({"format": FORMAT, "revoked": revoked, "grants": [{
        "id": "g-transition", "instance": INSTANCE, "operation": "issue.transition.run",
        "target": [
            {"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
            {"pointer": "/target_status",
             "one_of": ["10000", "10235", "3", "10234", "10223", "10001"]}
        ],
        "input_keys": ["/current_status", "/body/transition/id"],
        "not_after_unix_ms": not_after_unix_ms, "max_per_hour": max_per_hour
    }]})
}

/// One keyed write of `issue.transition.run` whose input is the JSON text
/// `input`, sent as these bytes.
async fn run(host: &Host, name: &str, input: &str) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let body = format!(
        r#"{{"version":"v1alpha2","request_id":"w-{name}","operation":"issue.transition.run","revision":"rev-1","idempotency_key":"key-{name}","input":{input}}}"#
    );
    let response = reqwest::Client::new()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .body(body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode_keyed(&bytes).unwrap())
}

/// The consumer's hop of `ticket`, as JSON text.
fn hop(ticket: &str) -> String {
    format!(
        r#"{{"issueIdOrKey":"{ticket}","current_status":"3","target_status":"10234","body":{{"transition":{{"id":"31"}}}}}}"#
    )
}

fn code(response: &InvokeResponse) -> Option<ErrorCode> {
    response.error.as_ref().map(|error| error.code)
}

/// Holds. The served half of acceptance 12 and 14, which the story tests
/// through the local `admit` only: one cap for every ticket the pattern
/// matches, no use for a refused write, and the end of the grant at
/// revocation and at `not_after`; and the forms of invariants 1 and 2 as
/// bytes on the wire.
#[tokio::test]
async fn a_served_pattern_grant_holds_one_cap_and_ends_as_an_exact_one() {
    let root = tempfile::tempdir().unwrap();
    let grants = root.path().join("grants.json");
    put(&grants, &transition_file(3, LATER, json!([])));
    let store = root.path().join("state");
    let host = start(root, grants.clone()).await;
    let rows = || {
        Uses::existing(&store)
            .unwrap()
            .map(|uses| uses.rows().unwrap())
    };

    // Outside the pattern or the set: refused, no use, no provider request.
    let outside = [
        ("ops", hop("OPS-12")),
        ("lower", hop("dev-12")),
        ("newline", hop(r"DEV-12\n")),
        ("newline-escape", hop(r"DEV-12\u000a")),
        ("carriage", hop(r"DEV-12\r")),
        ("suffix", hop("DEV-12x")),
        ("wide-digits", hop("DEV-\u{ff11}\u{ff12}")),
        (
            "number-key",
            r#"{"issueIdOrKey":12,"current_status":"3","target_status":"10234","body":{"transition":{"id":"31"}}}"#.to_owned(),
        ),
        (
            "list-key",
            r#"{"issueIdOrKey":["DEV-12"],"current_status":"3","target_status":"10234","body":{"transition":{"id":"31"}}}"#.to_owned(),
        ),
        (
            "number-status",
            r#"{"issueIdOrKey":"DEV-12","current_status":"3","target_status":10234,"body":{"transition":{"id":"31"}}}"#.to_owned(),
        ),
        (
            "exponent-status",
            r#"{"issueIdOrKey":"DEV-12","current_status":"3","target_status":1.0234e4,"body":{"transition":{"id":"31"}}}"#.to_owned(),
        ),
        (
            "other-status",
            r#"{"issueIdOrKey":"DEV-12","current_status":"3","target_status":"10002","body":{"transition":{"id":"31"}}}"#.to_owned(),
        ),
        (
            "fields",
            r#"{"issueIdOrKey":"DEV-12","current_status":"3","target_status":"10234","body":{"transition":{"id":"31"},"fields":{"labels":["x"]}}}"#.to_owned(),
        ),
    ];
    for (name, input) in &outside {
        let (status, refused) = run(&host, name, input).await;
        assert_eq!(
            (status, code(&refused)),
            (403, Some(ErrorCode::ApprovalRequired)),
            "{name}: {refused:?}"
        );
        assert!(
            refused.audit_ref.is_none() && refused.mutation.is_none(),
            "{name}"
        );
        assert_eq!(host.requests(), 0, "{name} reached the provider");
        assert_eq!(rows(), Some(0), "{name} took a use");
    }

    // Three tickets under the cap of 3, one counter.
    for (n, ticket) in ["DEV-1", "DEV-2", "DEV-3"].into_iter().enumerate() {
        let (status, admitted) = run(&host, ticket, &hop(ticket)).await;
        assert_eq!(
            (status, code(&admitted)),
            (200, None),
            "{ticket}: {admitted:?}"
        );
        assert_eq!(host.requests(), n + 1);
        assert_eq!(rows(), Some(n as u32 + 1));
    }
    // The next write to any ticket the pattern matches is at the cap.
    for (name, ticket) in [("fourth", "DEV-4"), ("first-again", "DEV-1")] {
        let (status, limited) = run(&host, name, &hop(ticket)).await;
        assert_eq!(
            (status, code(&limited)),
            (429, Some(ErrorCode::RateLimited)),
            "{name}: {limited:?}"
        );
        let retry = limited.error.unwrap().retry_after_seconds;
        assert!(matches!(retry, Some(1..=3_600)), "{name}: {retry:?}");
    }
    // At the cap a write outside the pattern is still outside the grant.
    let (status, refused) = run(&host, "ops-at-the-cap", &hop("OPS-12")).await;
    assert_eq!(
        (status, code(&refused)),
        (403, Some(ErrorCode::ApprovalRequired))
    );
    // A replay at the cap: the original outcome, no use.
    let (status, replay) = run(&host, "DEV-1", &hop("DEV-1")).await;
    assert_eq!((status, code(&replay)), (200, None), "{replay:?}");
    assert!(replay.mutation.unwrap().replayed);
    assert_eq!((host.requests(), rows()), (3, Some(3)));

    // Revoked, then expired: the pattern grant ends as an exact one does.
    put(
        &grants,
        &transition_file(100, LATER, json!(["g-transition"])),
    );
    let (status, refused) = run(&host, "revoked", &hop("DEV-5")).await;
    assert_eq!(
        (status, code(&refused)),
        (403, Some(ErrorCode::ApprovalRequired)),
        "{refused:?}"
    );
    put(&grants, &transition_file(100, 1, json!([])));
    let (status, refused) = run(&host, "expired", &hop("DEV-6")).await;
    assert_eq!(
        (status, code(&refused)),
        (403, Some(ErrorCode::ApprovalRequired)),
        "{refused:?}"
    );
    assert_eq!((host.requests(), rows()), (3, Some(3)));

    // A higher cap admits again, and the uses taken before still count:
    // back at 3, the four uses of the hour are over it.
    put(&grants, &transition_file(100, LATER, json!([])));
    let (status, admitted) = run(&host, "DEV-7", &hop("DEV-7")).await;
    assert_eq!((status, code(&admitted)), (200, None), "{admitted:?}");
    assert_eq!((host.requests(), rows()), (4, Some(4)));
    put(&grants, &transition_file(3, LATER, json!([])));
    let (status, limited) = run(&host, "DEV-8", &hop("DEV-8")).await;
    assert_eq!(
        (status, code(&limited)),
        (429, Some(ErrorCode::RateLimited)),
        "{limited:?}"
    );
    assert_eq!((host.requests(), rows()), (4, Some(4)));
}
