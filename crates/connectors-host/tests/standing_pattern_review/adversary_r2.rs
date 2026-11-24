//! Round 2 adversary review of
//! story:standing-grant-binds-a-pattern-or-a-value-set, after the review-fix
//! round: the reader of `local::standing` builds a pattern's matcher from the
//! lazy DFA and the PikeVM of `regex-automata` only, and refuses a pattern
//! that turns case-insensitive matching on while Unicode matching is on
//! (contracts/service/compatibility.md § 2.1; contracts/cli/v1alpha1/semantics.md,
//! "Standing grants"; docs/local-approvals.md, "A grant for many targets").
//!
//! Driven through the public surface both placements call: `standing::parse`,
//! `Grants::covering` and `standing::check_request`.
//!
//! - The fold refusal at its edges, against a second reading of the flags
//!   written here and against what a taken pattern admits.
//! - The matcher against `regex::Regex` on the wrapped pattern.
//! - The suite's own fold cases against mutants of the flag reading. The
//!   mutation lives here, in the test; no file of the tree is changed.
//! - What one read holds, against the figures the contract states.
#![cfg(unix)]
use crate::support::{LIVE, PEAK};
use connectors_host::local::standing::{self, Grants, Refused, Unreadable};
use regex_syntax::ast::{self, Ast};
use serde_json::{Value, json};
use std::{cell::Cell, time::Duration};

const INSTANCE: &str = "leaf";
const OPERATION: &str = "issue.transition.run";
const LATER: i64 = 2_000_000_000_000;

// ---------------------------------------------------------------------------
// The grant file and the grant request, as the two placements read them.
// ---------------------------------------------------------------------------

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
/// `{"pointer": "/k", "pattern": pattern}`: the file the contract's figures
/// were measured on (`security_r1.rs`, `file_of`).
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

/// What `approvals grant-standing`'s check answers for a request whose one
/// target member holds `pattern`.
fn requested(pattern: &str) -> Result<(), Refused> {
    let request = serde_json::to_vec(&json!({
        "operation": OPERATION,
        "target": [{"pointer": "/k", "pattern": pattern}],
        "not_after_unix_ms": LATER, "max_per_hour": 1
    }))
    .unwrap();
    standing::check_request(&request, INSTANCE)
}

/// The grants of a file whose one member holds `pattern`, or `None` when the
/// reader refuses the file. The request check must answer the same: a
/// pattern one of the two takes and the other refuses is a panic here.
fn taken(pattern: &str) -> Option<Grants> {
    let file = match standing::parse(&file_of(pattern, 1)) {
        Ok(grants) => Some(grants),
        Err(Unreadable::Malformed) => None,
        Err(other) => panic!("{pattern:?}: {other:?}"),
    };
    let request = requested(pattern);
    match (&file, &request) {
        (Some(_), Ok(())) | (None, Err(Refused::InvalidInput)) => {}
        _ => panic!(
            "{pattern:?}: the reader {} it and the request check answers {request:?}",
            if file.is_some() { "takes" } else { "refuses" }
        ),
    }
    file
}

fn admits(grants: &Grants, value: &str) -> bool {
    grants
        .covering(INSTANCE, OPERATION, &json!({"k": value}), 1_000)
        .is_ok()
}

// ---------------------------------------------------------------------------
// A second reading of the rule, written here: the form, the `regex` crate on
// the pattern and on its wrap, and the flags read by a recursive walk. Five
// mutants of the walk.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Reading {
    /// The flags as the translator of `regex-syntax` reads them, which
    /// `UnicodeCaseFold` says it follows: `(?flags)` holds to the end of the
    /// group it stands in, over a `|` too, and `(?flags:...)` holds inside
    /// that group.
    AsBuilt,
    /// Mutant: a `(?flags)` ends at the next `|` of its group.
    EndsAtABranch,
    /// Mutant: the end of a group puts Unicode matching back and leaves the
    /// case fold as the group had it.
    FoldOutlivesItsGroup,
    /// Mutant: the end of a group puts nothing back.
    NothingEndsWithAGroup,
    /// Mutant: the flags of `(?flags:...)` are followed and never judged.
    GroupFlagsUnjudged,
    /// Mutant: only a place that turns the case fold on is judged, so
    /// turning Unicode matching back on under a fold is not.
    UnicodeBackOnUnjudged,
}

/// Follows `flags` in `state` (case-insensitive, Unicode) and says whether
/// the place is one the rule refuses.
fn set(flags: &ast::Flags, state: &mut (bool, bool), reading: Reading) -> bool {
    let turned_on = flags.flag_state(ast::Flag::CaseInsensitive);
    if let Some(insensitive) = turned_on {
        state.0 = insensitive;
    }
    if let Some(unicode) = flags.flag_state(ast::Flag::Unicode) {
        state.1 = unicode;
    }
    match reading {
        Reading::UnicodeBackOnUnjudged => turned_on == Some(true) && state.1,
        _ => state.0 && state.1,
    }
}

/// Whether `node` holds a place where case-insensitive matching is on while
/// Unicode matching is on, under `reading`.
fn folds(node: &Ast, state: &mut (bool, bool), reading: Reading) -> bool {
    match node {
        Ast::Flags(item) => set(&item.flags, state, reading),
        Ast::Group(group) => {
            let outer = *state;
            let here = group
                .flags()
                .is_some_and(|flags| set(flags, state, reading))
                && reading != Reading::GroupFlagsUnjudged;
            let found = here || folds(&group.ast, state, reading);
            match reading {
                Reading::NothingEndsWithAGroup => {}
                Reading::FoldOutlivesItsGroup => state.1 = outer.1,
                _ => *state = outer,
            }
            found
        }
        Ast::Repetition(repetition) => folds(&repetition.ast, state, reading),
        Ast::Concat(concat) => {
            for ast in &concat.asts {
                if folds(ast, state, reading) {
                    return true;
                }
            }
            false
        }
        Ast::Alternation(alternation) => {
            let outer = *state;
            for ast in &alternation.asts {
                if reading == Reading::EndsAtABranch {
                    *state = outer;
                }
                if folds(ast, state, reading) {
                    return true;
                }
            }
            if reading == Reading::EndsAtABranch {
                *state = outer;
            }
            false
        }
        _ => false,
    }
}

fn wrapped(pattern: &str) -> String {
    format!(r"\A(?:{pattern})\z")
}

fn compiled(pattern: &str) -> Option<regex::Regex> {
    regex::RegexBuilder::new(pattern)
        .size_limit(64 * 1024)
        .dfa_size_limit(64 * 1024)
        .build()
        .ok()
}

/// The rule without the fold refusal, as the `regex` crate decides it: the
/// form, the pattern on its own and its wrap. The wrap's matcher.
fn elsewhere(pattern: &str) -> Option<regex::Regex> {
    if pattern.len() > 256
        || !pattern.starts_with('^')
        || !pattern.ends_with('$')
        || pattern.ends_with("\\$")
    {
        return None;
    }
    compiled(pattern)?;
    compiled(&wrapped(pattern))
}

/// Whether the rule takes `pattern` when the flags are read as `reading`
/// reads them.
fn expected(reading: Reading, pattern: &str) -> bool {
    if elsewhere(pattern).is_none() {
        return false;
    }
    let whole = ast::parse::Parser::new()
        .parse(&wrapped(pattern))
        .expect("the regex crate compiled it");
    !folds(&whole, &mut (false, true), reading)
}

// ---------------------------------------------------------------------------
// The fold refusal at its edges.
// ---------------------------------------------------------------------------

/// A pattern, whether the reader takes it, values it admits and values it
/// refuses.
type Edge = (
    &'static str,
    bool,
    &'static [&'static str],
    &'static [&'static str],
);

/// The patterns of the brief's list: flags inside groups and nested groups,
/// after an alternation, a fold turned off again, classes, `\p{..}` under a
/// fold, a flag group at the end, the `x` flag with comments.
const EDGES: &[Edge] = &[
    // A `(?flags)` holds over a `|` of its group.
    (
        "^(?i-u)s|k$",
        true,
        &["s", "S", "k", "K"],
        &["\u{17f}", "\u{212a}", "sk"],
    ),
    (
        "^(?:(?i-u)s)|k$",
        true,
        &["s", "S", "k"],
        &["K", "\u{212a}"],
    ),
    ("^(?i-u)s|(?u)k$", false, &[], &[]),
    (
        "^(?-u)s|(?i)k$",
        true,
        &["s", "k", "K"],
        &["S", "\u{212a}", "\u{17f}"],
    ),
    // A flag in the last branch of a group ends with the group.
    ("^(?:s|(?-u))(?i)k$", false, &[], &[]),
    (
        "^(?:s|(?-u)(?i)k)$",
        true,
        &["s", "k", "K"],
        &["S", "\u{212a}"],
    ),
    // Unicode back on with the fold off in the same flags: no fold place.
    (
        "^(?i-u)s(?u-i:k)$",
        true,
        &["sk", "Sk"],
        &["SK", "s\u{212a}", "\u{17f}k"],
    ),
    (
        "^(?i-u)s(?-i)(?u)k$",
        true,
        &["sk", "Sk"],
        &["SK", "s\u{212a}", "\u{17f}k"],
    ),
    // The conservative cases: a fold place that folds no letter.
    ("^(?i-u)s(?u)(?-i)k$", false, &[], &[]),
    ("^(?i)(?-u)sk$", false, &[], &[]),
    ("^(?i:(?-i:sk))$", false, &[], &[]),
    ("^(?i:(?-u:sk))$", false, &[], &[]),
    ("^sk(?i)$", false, &[], &[]),
    ("^(?i)[0-9]+$", false, &[], &[]),
    ("^(?iu)sk$", false, &[], &[]),
    // A flag group at the end, with Unicode off.
    ("^sk(?i-u)$", true, &["sk"], &["SK", "Sk"]),
    ("^(?i-u)$", true, &[""], &["i", "\u{17f}"]),
    // Nested groups.
    (
        "^(?-u:(?i:s(?u-i:k)))$",
        true,
        &["sk", "Sk"],
        &["SK", "\u{17f}k", "s\u{212a}"],
    ),
    ("^(?-u:(?i:s(?u:k)))$", false, &[], &[]),
    (
        "^((?i-u)s)k$",
        true,
        &["sk", "Sk"],
        &["SK", "sK", "\u{17f}k"],
    ),
    (
        "^(?:(?i-u)s){2}k$",
        true,
        &["ssk", "SSk", "sSk"],
        &["ssK", "s\u{17f}k"],
    ),
    (
        "^(?:|(?i-u)s)k$",
        true,
        &["k", "sk", "Sk"],
        &["K", "SK", "\u{17f}k"],
    ),
    ("^(?u)(?-u)(?i)s$", true, &["s", "S"], &["\u{17f}"]),
    // Classes, and a class that only looks like a flag.
    (
        "^(?i-u:[sk])+$",
        true,
        &["sk", "SkKs"],
        &["\u{17f}", "\u{212a}", "s\u{212a}"],
    ),
    (
        "^[(?i)k]$",
        true,
        &["k", "i", "(", "?", ")"],
        &["K", "\u{212a}", "I"],
    ),
    ("^(?i)[sk]$", false, &[], &[]),
    // `\p{..}` under a fold, and beside one.
    (r"^(?i)\p{Lu}$", false, &[], &[]),
    (r"^(?i)[\p{Greek}]$", false, &[], &[]),
    (r"^(?i:\pL)$", false, &[], &[]),
    (
        r"^\p{Lu}(?i-u)k$",
        true,
        &["Ak", "AK", "\u{c9}k"],
        &["ak", "A\u{212a}", "k"],
    ),
    // The `x` flag: a flag inside a comment is no flag, and without `x` it
    // is one.
    (
        "^(?x)(?i-u) s k # (?i)\n$",
        true,
        &["sk", "SK"],
        &["\u{17f}k", "s k", "sk # (?i)\n"],
    ),
    ("^(?x) s k # (?i)\n$", true, &["sk"], &["SK", "s\u{212a}"]),
    ("^(?x: s k # (?i)\n)$", true, &["sk"], &["SK", "\u{17f}k"]),
    ("^ s # (?i)\nk$", false, &[], &[]),
    ("^(?x)(?i) s k # (?-i)\n$", false, &[], &[]),
    (
        "^(?x) (?i-u) s # (?u)\n k $",
        true,
        &["sk", "SK"],
        &["s\u{212a}", "\u{17f}k"],
    ),
];

/// The brief's list for the fold refusal: each pattern is refused or taken as
/// the contract texts say ("A pattern that turns case-insensitive matching on
/// while Unicode matching is on ... is refused"; "`(?i-u)` and `(?i-u:...)`
/// fold ASCII letters only and are taken"), by the reader and by the request
/// check alike, and a taken one answers as `regex::Regex` does on its wrap.
#[test]
fn the_fold_refusal_holds_at_the_edges_of_the_flag_syntax() {
    for (pattern, takes, admitted, refused) in EDGES {
        assert_eq!(
            expected(Reading::AsBuilt, pattern),
            *takes,
            "{pattern:?}: the second reading"
        );
        let grants = taken(pattern);
        assert_eq!(grants.is_some(), *takes, "{pattern:?} taken");
        let Some(grants) = grants else { continue };
        let regex = elsewhere(pattern).expect("taken");
        for (values, answer) in [(admitted, true), (refused, false)] {
            for value in *values {
                assert_eq!(admits(&grants, value), answer, "{pattern:?} on {value:?}");
                assert_eq!(regex.is_match(value), answer, "{pattern:?} on {value:?}");
            }
        }
    }
    // The longest pattern with a fold: 256 bytes are taken, 257 are not.
    for (letters, takes) in [(248, true), (249, false)] {
        let pattern = format!("^(?i-u){}$", "a".repeat(letters));
        assert_eq!(pattern.len(), letters + 8);
        let grants = taken(&pattern);
        assert_eq!(grants.is_some(), takes, "{} bytes", pattern.len());
        if let Some(grants) = grants {
            assert!(admits(&grants, &"A".repeat(letters)));
            assert!(!admits(&grants, &"A".repeat(letters + 1)));
        }
    }
}

/// RED on the tree under review. The contract texts, after the fix round:
/// "`(?i-u)` and `(?i-u:...)` fold ASCII letters only and are taken"
/// (compatibility.md § 2.1, semantics.md, standing_approvals.yaml), and
/// docs/local-approvals.md: "A pattern may turn case-insensitive matching on
/// only where Unicode matching is off". With Unicode matching off, `.`,
/// `(?s:.)` and a negated class match any byte, which the `regex` crate
/// refuses in a pattern over text. So the consumer's comment pattern of the
/// same page, `^Release review of (?s:.*)$`, is refused once a signer puts
/// `(?i-u)` in front of it as the texts say, with `invalid_input` and no
/// word about why. Fix: the texts say the fold goes around the letters only,
/// `^(?i-u:release review of )(?s:.*)$`, and that `.` and a negated class
/// do not stand under `-u`.
#[test]
fn a_fold_with_unicode_off_is_taken_as_the_texts_say() {
    // The control: the spelling that works, and the ticket pattern.
    let around = taken("^(?i-u:release review of )(?s:.*)$").expect("taken");
    assert!(admits(&around, "RELEASE REVIEW OF 2.46.8\nsecond line"));
    assert!(!admits(&around, "Note: release review of 2.46.8"));
    assert!(taken("^(?i-u)DEV-[0-9]+$").is_some());
    // Changed in fix round 2 (decision D-b): the reader stays and the texts
    // change. This case asserted that the four patterns on the left are
    // taken, as the texts then said. The texts now say that a fold is written
    // as a group around the letters only and that `.`, a negated class and
    // `\W` are refused in byte mode, so each pattern on the left is refused
    // and its spelling on the right is taken; and each of the five texts is
    // read here, so the case is red again on a text that says a fold with
    // Unicode matching off is taken without the byte-mode rule.
    for (in_byte_mode, around_the_letters) in [
        (
            "^(?i-u)Release review of (?s:.*)$",
            "^(?i-u:release review of )(?s:.*)$",
        ),
        (
            "^(?i-u)Release review of .*$",
            "^(?i-u:release review of ).*$",
        ),
        ("^(?i-u)DEV-[^/]+$", "^(?i-u:dev)-[^/]+$"),
        ("^(?i-u:dev-.+)$", "^(?i-u:dev-).+$"),
    ] {
        assert!(taken(in_byte_mode).is_none(), "{in_byte_mode}");
        assert!(taken(around_the_letters).is_some(), "{around_the_letters}");
    }
    // Added in fix round 3 (security review of round 3, F3): the texts named
    // `.`, a negated class and `\W` only. Each class the texts now name
    // beside them is refused behind a bare `(?i-u)` and inside the group,
    // the difference they name is taken and admits ASCII only, and each of
    // the five texts is held to the clause below.
    for class in [r"\D", r"\S", "[[:^alpha:]]", r"[^\W]"] {
        for refused in [format!("^(?i-u){class}$"), format!("^(?i-u:{class})$")] {
            assert!(taken(&refused).is_none(), "{refused}");
        }
    }
    let difference = taken(r"^(?i-u)[\x00-\x7f--a]$").expect("the texts call it taken");
    assert!(admits(&difference, "b") && admits(&difference, "/"));
    for outside in ["a", "A", "\u{e9}", "\u{80}"] {
        assert!(!admits(&difference, outside), "{outside:?}");
    }
    for (name, text) in [
        (
            "contracts/service/compatibility.md",
            include_str!("../../../../contracts/service/compatibility.md"),
        ),
        (
            "contracts/cli/v1alpha1/semantics.md",
            include_str!("../../../../contracts/cli/v1alpha1/semantics.md"),
        ),
        (
            "docs/local-approvals.md",
            include_str!("../../../../docs/local-approvals.md"),
        ),
        (
            "docs/running-services.md",
            include_str!("../../../../docs/running-services.md"),
        ),
        (
            "ess/domains/standing_approvals.yaml",
            include_str!("../../../../ess/domains/standing_approvals.yaml"),
        ),
    ] {
        // One line, without the `#` of a YAML comment.
        let text = text
            .split_whitespace()
            .filter(|word| *word != "#")
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !text.contains("`(?i-u)` and `(?i-u:...)` fold ASCII letters only and are taken"),
            "{name} says every fold with Unicode matching off is taken"
        );
        for rule in [
            "as a group around the letters only",
            "the pattern is in byte mode",
            "a negated class such as `[^/]` and `\\W`",
            "`^(?i-u)dev$` is taken",
            "class that can match a byte outside ASCII",
            "also one written inside another class",
            "`\\D`, `\\S`, a negated POSIX class such as `[[:^alpha:]]` and `[^\\W]`",
            "a difference from an ASCII class such as `[\\x00-\\x7f--a]` is taken",
        ] {
            assert!(text.contains(rule), "{name} does not say: {rule}");
        }
    }
}

// ---------------------------------------------------------------------------
// Generated patterns, with a fixed seed.
// ---------------------------------------------------------------------------

struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn pick<'a>(&mut self, of: &[&'a str]) -> &'a str {
        of[self.below(of.len())]
    }

    /// A sequence of atoms, `(?flags)`, `|` and groups, nested to `depth`.
    fn body(&mut self, atoms: &[&str], suffixes: &[&str], flags: &[&str], depth: usize) -> String {
        let mut out = String::new();
        for _ in 0..1 + self.below(4) {
            match self.below(if depth < 3 { 11 } else { 7 }) {
                0..=2 => {
                    out.push_str(self.pick(atoms));
                    out.push_str(self.pick(suffixes));
                }
                3..=5 => {
                    out.push_str("(?");
                    out.push_str(self.pick(flags));
                    out.push(')');
                }
                6 => out.push('|'),
                7 | 8 => {
                    out.push_str("(?");
                    out.push_str(self.pick(flags));
                    out.push(':');
                    out.push_str(&self.body(atoms, suffixes, flags, depth + 1));
                    out.push(')');
                    out.push_str(self.pick(suffixes));
                }
                9 => {
                    out.push('(');
                    out.push_str(&self.body(atoms, suffixes, flags, depth + 1));
                    out.push(')');
                }
                _ => {
                    out.push_str("(?:");
                    out.push_str(&self.body(atoms, suffixes, flags, depth + 1));
                    out.push(')');
                    out.push_str(self.pick(suffixes));
                }
            }
        }
        out
    }
}

/// The look-alikes of the story: U+017F folds to `s` and U+212A to `k`.
const LOOK_ALIKES: [char; 2] = ['\u{17f}', '\u{212a}'];

/// Patterns whose only letters are ASCII `s` and `k` in either case, under
/// every placement of the case and Unicode flags the generator reaches
/// (2,500 patterns, seed fixed). For each:
/// - the reader and the request check take it exactly when the second
///   reading of the rule does;
/// - a taken pattern answers every value as `regex::Regex` answers on the
///   wrap, and admits no value that holds a look-alike: no ASCII class or
///   letter of these patterns matches one without a Unicode case fold.
#[test]
fn a_generated_pattern_is_refused_exactly_when_it_reaches_the_unicode_fold() {
    let atoms = [
        "s",
        "k",
        "S",
        "K",
        "sk",
        "[sk]",
        "[a-z]",
        "[K-S]",
        " # (?i)\n",
        " ",
    ];
    let suffixes = ["", "", "", "", "?", "*", "+", "{2}"];
    let flags = [
        "i", "i", "-i", "u", "-u", "-u", "i-u", "i-u", "u-i", "-iu", "iu", "s", "x", "m", "is",
        "ix-u", "x-u", "si-u", "U", "-x",
    ];
    let letters = ["s", "k", "S", "K", "\u{17f}", "\u{212a}"];
    let mut values = vec![String::new()];
    for first in letters {
        values.push(first.to_owned());
        for second in letters {
            values.push(format!("{first}{second}"));
        }
    }
    for longer in [
        "sks",
        "SKS",
        "s\u{212a}s",
        "\u{17f}ks",
        "kkss",
        "SkSk\u{212a}",
    ] {
        values.push(longer.to_owned());
    }

    let mut generator = Generator(0x5eed_0002);
    let (mut takes, mut fold_refused, mut elsewhere_refused, mut folded_ascii) = (0, 0, 0, 0);
    for _ in 0..2_500 {
        let pattern = format!("^{}$", generator.body(&atoms, &suffixes, &flags, 0));
        let grants = taken(&pattern);
        assert_eq!(
            grants.is_some(),
            expected(Reading::AsBuilt, &pattern),
            "{pattern:?} taken"
        );
        let Some(regex) = elsewhere(&pattern) else {
            elsewhere_refused += 1;
            continue;
        };
        let Some(grants) = grants else {
            fold_refused += 1;
            continue;
        };
        takes += 1;
        let mut folded = false;
        for value in &values {
            let answer = admits(&grants, value);
            assert_eq!(answer, regex.is_match(value), "{pattern:?} on {value:?}");
            assert!(
                !(answer && value.contains(LOOK_ALIKES)),
                "{pattern:?} admits {}",
                value.escape_unicode()
            );
            // The same value in lower case is admitted too: an ASCII fold.
            let lower = value.to_ascii_lowercase();
            folded |= answer && lower != *value && admits(&grants, &lower);
        }
        folded_ascii += usize::from(folded);
    }
    // The generator reaches all three answers, and taken patterns that fold.
    assert!(
        takes >= 300 && fold_refused >= 300 && elsewhere_refused >= 50 && folded_ascii >= 50,
        "taken {takes}, refused for the fold {fold_refused}, refused by the regex crate \
         {elsewhere_refused}, taken and folding ASCII on some value {folded_ascii}"
    );
}

/// The matcher of the fix round against `regex::Regex` on the wrapped
/// pattern, over patterns of the syntax a signer writes (classes,
/// Unicode-aware escapes, word boundaries, anchors inside, counted and lazy
/// repetitions, every flag) and over values of ASCII, other scripts, line
/// breaks and a four-byte character (3,000 patterns, seed fixed). The
/// reader takes a pattern exactly when the second reading does, and a taken
/// one answers every value as the `regex` crate does.
#[test]
fn a_generated_pattern_matches_what_the_regex_crate_matches_on_its_wrap() {
    let atoms = [
        "a",
        "b",
        "A",
        "1",
        "-",
        "_",
        ".",
        r"\w",
        r"\d",
        r"\s",
        r"\b",
        r"\B",
        "[^a]",
        "[a-c]",
        r"\p{Greek}",
        r"\PL",
        "\u{e9}",
        "\u{3b1}",
        "$",
        "^",
        r"\z",
        r"\A",
        r"[\d&&[^5]]",
        " ",
        "\n",
        r"\x{1f4a9}",
        r"[\w--\d]",
        r"\W",
        "(?s:.)",
        "ab",
        "\u{17f}",
        "k",
    ];
    let suffixes = [
        "", "", "", "", "?", "*", "+", "{2}", "*?", "+?", "{1,2}", "{0}", "??",
    ];
    let flags = [
        "i-u", "i-u", "-u", "u", "s", "m", "x", "R", "U", "-i", "s-u", "i", "u-i", "im-u", "sm",
        "-s", "-m", "x-u",
    ];
    let alphabet = [
        "a",
        "b",
        "A",
        "B",
        "1",
        "5",
        "\u{663}",
        "\u{e9}",
        "\u{c9}",
        "\u{3b1}",
        "\u{391}",
        " ",
        "\n",
        "\r",
        "_",
        "-",
        "\u{1f4a9}",
        "\u{17f}",
        "\u{212a}",
        "k",
        "ab",
    ];
    let mut generator = Generator(0x5eed_0003);
    let mut values: Vec<String> = ["", "a", "\n", "a\n", "\r\n", "ab", "\u{e9}", "a b", "1-1"]
        .map(str::to_owned)
        .to_vec();
    for _ in 0..70 {
        let mut value = String::new();
        for _ in 0..generator.below(6) {
            value.push_str(generator.pick(&alphabet));
        }
        values.push(value);
    }

    let (mut takes, mut refuses, mut admitted) = (0, 0, 0);
    for _ in 0..3_000 {
        let pattern = format!("^{}$", generator.body(&atoms, &suffixes, &flags, 0));
        let grants = taken(&pattern);
        assert_eq!(
            grants.is_some(),
            expected(Reading::AsBuilt, &pattern),
            "{pattern:?} taken"
        );
        let Some(grants) = grants else {
            refuses += 1;
            continue;
        };
        takes += 1;
        let regex = elsewhere(&pattern).expect("taken");
        for value in &values {
            let answer = admits(&grants, value);
            assert_eq!(answer, regex.is_match(value), "{pattern:?} on {value:?}");
            admitted += usize::from(answer);
        }
    }
    assert!(
        takes >= 500 && refuses >= 500 && admitted >= 2_000,
        "taken {takes}, refused {refuses}, admitted values {admitted}"
    );
}

// ---------------------------------------------------------------------------
// The suite's own fold cases against mutants of the flag reading.
// ---------------------------------------------------------------------------

/// Every pattern with a flag that the story's tests and the two reviews put
/// to the reader, with whether they assert it taken:
/// `local::standing::tests::a_standing_pattern_folds_case_only_with_unicode_off`
/// (both lists), `adversary_r1.rs`
/// (`a_case_insensitive_pattern_admits_no_unicode_look_alike`,
/// `an_ascii_case_fold_is_written_with_the_unicode_flag_off`,
/// `a_flag_or_a_comment_inside_a_pattern_opens_nothing`,
/// `one_anchor_per_branch_and_a_multi_line_flag_still_hold_the_whole_value`),
/// `security_r1.rs` and the shared vectors.
fn suite_cases() -> Vec<(String, bool)> {
    let refused = [
        "^(?i)dev$",
        "^(?i:dev)$",
        "^(?i)$",
        "^(?si)dev$",
        "^(?i-s)dev$",
        "^(?u)(?i)dev$",
        "^D(?i)ev$",
        "^(d)(?i)ev$",
        "^(?:d|(?i)e)v$",
        "^d|(?i)ev$",
        "^(d(?i:e))v$",
        "^(?P<i>d)(?i:e)v$",
        "^(?-u:d)(?i)ev$",
        "^(?-u:(?i)d)(?i)ev$",
        "^(?i-u)de(?u)v$",
        "^(?i-u)de(?u:v)$",
        "^(?i-u:d(?u:e))v$",
        "^(?-u)(?i)d(?u)ev$",
        "^(?i)sk-[0-9]+$",
        "^(?i)[a-z]+-[0-9]+$",
        "^(?x)a#$",
        "^(?x)a\n#$",
        "^a|(?x)#$",
        "^(?x)a #$",
        "^a)|(?:b$",
        "(?m)^a$",
    ];
    let takes = [
        "^(?i-u)dev$",
        "^(?i-u:dev)$",
        "^(?-u)(?i)dev$",
        "^(?-u:(?i)dev)$",
        "^(?-u:(?i)de)v$",
        "^(?i-u:de)(?-i)v$",
        "^(?i-u)sk-[0-9]+$",
        "^(?-i)dev$",
        "^(?-i:dev)$",
        "^(?s-i:d.v)$",
        "^[(?i)]$",
        r"^\(\?i\)dev$",
        "^(?P<i>dev)$",
        "^(?i-u)[a-z]+-[0-9]+$",
        "^(?i-u:sk)-[0-9]+$",
        "^(?x: a # c\n)$",
        "^(?m)a$",
        "^(?m:a$\n^b)$",
        "^(?s).*$",
        "^(?R)a$",
        "^(?U)a+$",
        "^(?-u:[a-z])+$",
        "^(?m)$",
        "^(?m)DEV-[0-9]+$",
        "^(?m:^DEV-[0-9]+$)$",
        "^(?R)DEV-[0-9]+$",
        "^(?U)DEV-[0-9]+$",
        "^Release review of (?s:.*)$",
        "^a|b$",
        "^a$|^b$",
        "^DEV-[0-9]+|x$",
        "^|$",
    ];
    let mut cases: Vec<(String, bool)> = refused
        .into_iter()
        .map(|pattern| (pattern.to_owned(), false))
        .chain(takes.into_iter().map(|pattern| (pattern.to_owned(), true)))
        .collect();
    cases.push((format!("^(?i)[{}]$", r"\pL".repeat(82)), false));
    let shared: Value =
        serde_json::from_str(include_str!("../fixtures/whole_value_patterns.json")).unwrap();
    for vector in shared["vectors"].as_array().unwrap() {
        cases.push((
            vector["pattern"].as_str().unwrap().to_owned(),
            !vector["matches"].is_null(),
        ));
    }
    cases
}

/// The cases of `cases` that `reading` answers otherwise than the case
/// asserts: the cases that turn a suite red under the mutant.
fn killers(reading: Reading, cases: &[(String, bool)]) -> Vec<&str> {
    cases
        .iter()
        .filter(|(pattern, takes)| expected(reading, pattern) != *takes)
        .map(|(pattern, _)| pattern.as_str())
        .collect()
}

/// The control of the cases below: the second reading answers every case of
/// the suite as the suite asserts it, and the tree answers the same.
#[test]
fn the_second_reading_is_the_reading_as_built_on_the_suites_cases() {
    let cases = suite_cases();
    assert!(cases.len() >= 90, "{}", cases.len());
    assert_eq!(killers(Reading::AsBuilt, &cases), Vec::<&str>::new());
    for (pattern, takes) in &cases {
        assert_eq!(taken(pattern).is_some(), *takes, "{pattern:?}");
    }
}

/// `UnicodeCaseFold`: "`(?flags)` holds to the end of the group it stands
/// in, over a `|` too". No case of the suite tells whether it does: under a
/// reading in which a `(?flags)` ends at the next `|`, every case of the
/// suite is answered as asserted. That reading takes `^(?i-u)s|(?u)k$`,
/// whose second branch folds `k` with Unicode on and admits U+212A. Fix: the
/// cases of `a_flag_holds_over_a_branch_and_a_fold_ends_with_its_group` in
/// the suite (`a_standing_pattern_folds_case_only_with_unicode_off`).
#[test]
fn a_suite_case_tells_whether_a_flag_holds_over_a_branch() {
    let cases = suite_cases();
    assert!(
        !killers(Reading::EndsAtABranch, &cases).is_empty(),
        "none of {} cases tells a `(?flags)` that holds over a `|` from one that ends at it",
        cases.len()
    );
}

/// `UnicodeCaseFold::visit_post` puts both flags back at the end of a group.
/// No case of the suite tells whether the case fold is put back: under a
/// reading in which the end of a group restores Unicode matching only, every
/// case of the suite is answered as asserted. That reading refuses
/// `^(?i-u:s)(?s:k)$`, which the contract texts say is taken.
#[test]
fn a_suite_case_tells_whether_the_fold_ends_with_its_group() {
    let cases = suite_cases();
    assert!(
        !killers(Reading::FoldOutlivesItsGroup, &cases).is_empty(),
        "none of {} cases tells whether the case fold ends with the group that set it",
        cases.len()
    );
}

/// Holds. The other three mutants each answer a case of the suite wrongly.
#[test]
fn the_suite_tells_the_other_readings_from_the_one_built() {
    let cases = suite_cases();
    for reading in [
        Reading::NothingEndsWithAGroup,
        Reading::GroupFlagsUnjudged,
        Reading::UnicodeBackOnUnjudged,
    ] {
        assert!(!killers(reading, &cases).is_empty(), "{reading:?}");
    }
}

/// Holds. The cases the two red ones ask for: the tree answers them as the
/// translator reads the flags, and each of the two mutants answers at least
/// one of them wrongly.
#[test]
fn a_flag_holds_over_a_branch_and_a_fold_ends_with_its_group() {
    let cases: Vec<(String, bool)> = [
        ("^(?i-u)s|(?u)k$", false),
        ("^(?-u)s|(?i)k$", true),
        ("^(?:s|(?-u))(?i)k$", false),
        ("^(?i-u:s)(?s:k)$", true),
        ("^(?-u:(?i)s)(?u)k$", true),
    ]
    .into_iter()
    .map(|(pattern, takes)| (pattern.to_owned(), takes))
    .collect();
    for (pattern, takes) in &cases {
        assert_eq!(taken(pattern).is_some(), *takes, "{pattern:?}");
    }
    assert_eq!(killers(Reading::AsBuilt, &cases), Vec::<&str>::new());
    assert!(!killers(Reading::EndsAtABranch, &cases).is_empty());
    assert!(!killers(Reading::FoldOutlivesItsGroup, &cases).is_empty());
    // What the flag over the branch decides: `k` folds ASCII only.
    let grants = taken("^(?-u)s|(?i)k$").expect("taken");
    assert!(admits(&grants, "K") && admits(&grants, "k") && admits(&grants, "s"));
    assert!(!admits(&grants, "\u{212a}") && !admits(&grants, "S"));
    // What the mutant would let in: the wrap of the refused pattern admits
    // the Kelvin sign.
    let open = regex::Regex::new(&wrapped("^(?i-u)s|(?u)k$")).unwrap();
    assert!(open.is_match("\u{212a}"));
    // The fold ends with its group: the `k` after it is matched as written.
    let grants = taken("^(?i-u:s)(?s:k)$").expect("taken");
    assert!(admits(&grants, "Sk") && !admits(&grants, "SK"));
}

// ---------------------------------------------------------------------------
// What one read holds.
// ---------------------------------------------------------------------------

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

/// One read of `file` and the check of the write `{"k": value}` against it,
/// as the contract measured: the bytes this thread holds once the write was
/// checked, how many grants cover the write, and the CPU time of the read
/// and of the check.
fn one_read(file: &[u8], value: &str) -> (isize, usize, Duration, Duration) {
    let input = json!({"k": value});
    let (before, started) = (live(), cpu());
    let grants = standing::parse(file).expect("a file within the stated limits");
    let read = cpu() - started;
    let started = cpu();
    let covering = grants
        .covering(INSTANCE, OPERATION, &input, 1_000)
        .map_or(0, |covering| covering.len());
    let checked = cpu() - started;
    let holds = live() - before;
    drop(grants);
    (holds, covering, read, checked)
}

/// compatibility.md § 2.1: "a file of 64 such members 3,950,582 bytes
/// (3.77 MiB)".
///
/// Changed in fix round 2 together with the contract's text: the figure was
/// 21,536,886 bytes, what 64 members of `^\ba?x{2034}\b$` held under the
/// matcher that round replaced (the lazy DFA and the PikeVM of
/// `regex_automata::meta`).
const STATED_ONE_READ: isize = 3_950_582;

/// compatibility.md § 2.1: "no more than 5 MiB at any moment", the rounded
/// bound on what one read and one checked write hold (fix round 2).
const STATED_BOUND: isize = 5 * 1024 * 1024;

/// The contract's costliest pattern and the value it was measured on: 113
/// times one class of 64 byte ranges, the characters of odd code point from
/// U+0001 to U+007F.
///
/// Changed in fix round 2: it was `^\ba?x{2034}\b$` on 2,034 times `x`,
/// which the reader now matches against 139 bytes and which holds less
/// ([`WORD_RUN`]).
const COSTLIEST: &str = r"^[\x01\x03\x05\x07\x09\x0b\x0d\x0f\x11\x13\x15\x17\x19\x1b\x1d\x1f!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{}\x7f]{113}$";

fn costliest_value() -> String {
    "a".repeat(113)
}

/// The costliest pattern the contract named before fix round 2.
const WORD_RUN: &str = r"^\ba?x{2034}\b$";

/// The CPUs this thread may run on, as `regex-automata` counts them when it
/// sizes the pool of a `meta` matcher: 64 bytes a CPU, for every pattern.
fn cpus() -> isize {
    std::thread::available_parallelism().map_or(1, |cpus| cpus.get() as isize)
}

/// Holds. compatibility.md § 2.1 states five figures as measured "as the
/// bytes held after one read of the file and one checked write, the document
/// included". Each is measured here on every CPU the host gives the process.
///
/// Changed in fix round 2: the figures are those of the PikeVM alone, and
/// the costliest pattern is the class run. Each held within 2 % of a figure
/// from which 64 bytes a pattern were taken off for every CPU fewer than the
/// 64 the old figures were measured on; now each holds within 2 % and within
/// 4,096 bytes of the stated figure itself, on any CPU count. The next case
/// holds the same figure to the same 4,096 bytes on one CPU, so a matcher
/// that held 64 bytes a pattern and CPU would fail one of the two on every
/// host of two CPUs or more (8,192 bytes for a file of 64 patterns).
#[test]
fn one_read_holds_each_figure_the_contract_states() {
    let long = costliest_value();
    for (pattern, members, value, stated) in [
        (COSTLIEST, 1, long.as_str(), 61_727),
        (COSTLIEST, 64, long.as_str(), STATED_ONE_READ),
        (r"^\b\w\b$", 64, "a", 1_194_550),
        (
            "^Release review of (?s:.*)$",
            64,
            "Release review of 2.46.8",
            113_910,
        ),
        ("^DEV-[0-9]+$", 64, "DEV-12", 76_086),
    ] {
        let (holds, covering, _, _) = one_read(&file_of(pattern, members), value);
        assert_eq!(covering, members, "{pattern}");
        assert!(
            (holds - stated).abs() <= (stated / 50).min(4_096),
            "one read of {members} members of {pattern} holds {holds} bytes on {} CPUs; the \
             contract states {stated} for any host",
            cpus()
        );
        assert!(holds <= STATED_BOUND, "{pattern}: {holds}");
    }
}

/// Runs `run` with this thread bound to one of its CPUs.
#[cfg(target_os = "linux")]
fn on_one_cpu<T>(run: impl FnOnce() -> T) -> T {
    let size = std::mem::size_of::<libc::cpu_set_t>();
    // SAFETY: both sets are valid `cpu_set_t` values of `size` bytes, and the
    // calls read and write this thread's own mask only.
    unsafe {
        let mut all: libc::cpu_set_t = std::mem::zeroed();
        assert_eq!(libc::sched_getaffinity(0, size, &mut all), 0);
        let first = (0..libc::CPU_SETSIZE as usize)
            .find(|cpu| libc::CPU_ISSET(*cpu, &all))
            .expect("a CPU to run on");
        let mut one: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(first, &mut one);
        assert_eq!(libc::sched_setaffinity(0, size, &one), 0);
        let out = run();
        assert_eq!(libc::sched_setaffinity(0, size, &all), 0);
        out
    }
}

/// RED on the tree under review. The contract states its memory figures
/// without a host: "holds 336,513 bytes as the one member of a file, and a
/// file of 64 such members 21,536,886 bytes". The matcher's pool
/// (`regex_automata::util::pool`) holds 64 bytes for every CPU the process
/// may run on, in every pattern: 4,096 bytes a CPU for a file of 64
/// patterns. It counts the CPUs once in a process
/// (`std::thread::available_parallelism`), so this case binds its thread to
/// one CPU before the first pattern is built; the runner gives each case its
/// own process. The stated figures are those of a host of 64 CPUs. On one
/// CPU the read holds 258,048 bytes less; above 128 CPUs it holds more than
/// the band of the suite's own case
/// (`the_costliest_pattern_of_the_contract_holds_what_the_contract_states`,
/// 262,144 bytes), and that case fails. Fix: the texts name the CPU count
/// the figures hold for and the 64 bytes a pattern and CPU, and the suite's
/// case takes the CPU count into its figure.
///
/// Fix round 2: the reader matches with the PikeVM alone, which has no pool,
/// so the figure is the same on one CPU and on 64 (measured on a host of 64:
/// 3,950,582 bytes both ways). The assertion is as it was; the stated figure
/// and the pattern are the new ones.
#[cfg(target_os = "linux")]
#[test]
fn one_read_holds_the_stated_figure_whatever_the_hosts_cpu_count() {
    let (file, value) = (file_of(COSTLIEST, 64), costliest_value());
    let (holds, counted) = on_one_cpu(|| (one_read(&file, &value).0, cpus()));
    assert_eq!(counted, 1, "the thread is bound to one CPU");
    assert!(
        (holds - STATED_ONE_READ).abs() <= 4_096,
        "one read of 64 members of {COSTLIEST} holds {holds} bytes in a process that may run on \
         1 CPU; the contract states {STATED_ONE_READ} bytes and names no host"
    );
}

/// The pattern `make(n)` of the largest `n` the reader takes.
fn largest(make: impl Fn(usize) -> (String, String)) -> (String, String) {
    let takes = |n: usize| {
        let pattern = make(n).0;
        pattern.len() <= 256 && standing::parse(&file_of(&pattern, 1)).is_ok()
    };
    assert!(takes(1), "{}", make(1).0);
    let (mut low, mut high) = (1, 4_096);
    while low + 1 < high {
        let middle = (low + high) / 2;
        if takes(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    make(low)
}

/// `value` cut to the longest value the first member of `file` is matched
/// against: its first and its last character with `x` between them. A value
/// within the bound is returned as it is.
fn within_the_bound(file: &[u8], value: &str) -> String {
    let bound = longest(file);
    if value.len() <= bound {
        return value.to_owned();
    }
    let first = value.chars().next().expect("a value");
    let last = value.chars().next_back().expect("a value");
    let cut = format!(
        "{first}{}{last}",
        "x".repeat(bound - first.len_utf8() - last.len_utf8())
    );
    assert_eq!(cut.len(), bound);
    cut
}

/// One read holds no more than the contract's costliest pattern when the
/// value is one the lazy DFA does not finish: a character outside ASCII
/// under a Unicode word boundary, where the matcher of last resort, the
/// PikeVM, runs after the lazy DFA gave up. Each family is taken at the
/// largest size the reader takes. The bound is what the costliest pattern
/// holds on its own value in this process, and 65,536 bytes.
///
/// Changed in fix round 2 (the PikeVM is the one matcher, and a value longer
/// than `max_value_bytes` is not matched at all): each family is measured on
/// its value as before and on that value cut to the pattern's bound, so the
/// matcher runs on it. The first three cases keep the pattern they were
/// written for ([`WORD_RUN`]; they named `COSTLIEST`, which was that
/// pattern). The bound stays, against the new costliest pattern, and a
/// second one is added that does not move with it: no value makes a file
/// hold over 65,536 bytes more than the same file holds after a check of
/// the one-byte value `x`. A matcher that keeps the cache of a check holds
/// more than the stated figure for the costliest pattern, which is asserted
/// first; one whose held bytes grow with the value, as the lazy DFA's cache
/// did, fails the second bound.
#[test]
fn one_read_holds_no_more_when_the_matcher_of_last_resort_runs() {
    let stated = one_read(&file_of(COSTLIEST, 64), &costliest_value()).0;
    assert!(
        (stated - STATED_ONE_READ).abs() <= 4_096,
        "64 members of {COSTLIEST} hold {stated} bytes; the contract states {STATED_ONE_READ}"
    );
    let e = '\u{e9}';
    let mut cases = vec![
        (WORD_RUN.to_owned(), format!("{}{e}", "x".repeat(2_033))),
        (WORD_RUN.to_owned(), format!("{e}{}", "x".repeat(2_033))),
        (WORD_RUN.to_owned(), "x".repeat(2_035)),
    ];
    // The lazy DFA fills its cache, gives up at the last character, and the
    // PikeVM matches the whole value.
    cases.push(largest(|n| {
        (
            format!("^\\bx{{{n}}}{e}?\\b$"),
            format!("{}{e}", "x".repeat(n)),
        )
    }));
    cases.push(largest(|n| {
        (
            format!("^\\bx{{{n}}}\\w?\\b$"),
            format!("{}{e}", "x".repeat(n)),
        )
    }));
    // The lazy DFA gives up at the first character.
    cases.push(largest(|n| {
        (
            format!("^\\b{e}?x{{{n}}}\\b$"),
            format!("{e}{}", "x".repeat(n)),
        )
    }));
    // A search of at most 128 bytes, which `regex-automata` would give to
    // the bounded backtracker were it on, and the same pattern on a long one.
    let optional = largest(|n| (format!("^\\b{e}?x{{0,{n}}}\\b$"), String::new())).0;
    cases.push((optional.clone(), format!("{e}{}", "x".repeat(124))));
    cases.push((optional.clone(), "x".repeat(126)));
    cases.push((optional, format!("{e}{}", "x".repeat(900))));
    // Each case on its value and on that value cut to the pattern's bound.
    let cases: Vec<(String, String)> = cases
        .into_iter()
        .flat_map(|(pattern, value)| {
            let cut = within_the_bound(&file_of(&pattern, 64), &value);
            let both = if cut == value {
                vec![value]
            } else {
                vec![value, cut]
            };
            both.into_iter()
                .map(move |value| (pattern.clone(), value))
                .collect::<Vec<_>>()
        })
        .collect();
    let measured: Vec<(isize, isize, String)> = cases
        .iter()
        .map(|(pattern, value)| {
            let file = file_of(pattern, 64);
            let plain = one_read(&file, "x").0;
            let (holds, covering, _, _) = one_read(&file, value);
            let line = format!(
                "{holds} bytes ({:+} to the costliest, {:+} to its own check of `x`) covering \
                 {covering}: {pattern} on {} bytes",
                holds - stated,
                holds - plain,
                value.len()
            );
            (holds, plain, line)
        })
        .collect();
    let over: Vec<&String> = measured
        .iter()
        .filter(|(holds, plain, _)| *holds > stated + 65_536 || *holds > plain + 65_536)
        .map(|(_, _, line)| line)
        .collect();
    assert!(
        over.is_empty(),
        "64 members of {COSTLIEST} hold {stated} bytes here; of {} files, these hold more than \
         that and 65,536 bytes, or 65,536 bytes more than after a check of `x`: {over:#?}\nall: \
         {measured:#?}",
        measured.len()
    );
}

/// RED on the tree under review. Invariant 6 of the security review, memory
/// (`one_read_of_a_file_within_the_stated_limits_holds_no_more_than_the_contract_states`):
/// a file within every stated limit holds, in one read and one checked
/// write, no more than the contract states for "the costliest pattern
/// found". A pattern that starts with `(?s:.*)` holds more: every state of
/// its lazy DFA holds the whole run of `x`. Measured on a host of 64 CPUs:
/// 21,692,734 bytes against the stated 21,536,886 (155,848 bytes, 0.72 %,
/// over; inside the band of 262,144 bytes the suite's own case allows). The
/// bound here is what the contract's costliest pattern holds in this
/// process, so the case does not depend on the host. Fix: the texts name
/// this pattern and its figure, or state the figure as rounded.
///
/// Changed in fix round 2 (the work bound): the reader matches this pattern
/// against 141 bytes, so the value of 2,004 times `x` is inside no grant
/// (the case asserted 64 covering grants for it). It is still measured, and
/// so are the value `b`, which is inside all 64, and a run of `x` of the
/// pattern's bound, on which the matcher runs. The bound on what the read
/// holds is as it was. The same holds for the leading any-run before the
/// class of the costliest pattern's family, at the largest size the reader
/// takes, on a value inside all 64 grants.
#[test]
fn one_read_holds_no_more_than_the_contract_states_under_a_leading_any_run() {
    let stated = one_read(&file_of(COSTLIEST, 64), &costliest_value()).0;
    let pattern = "^(?s:.*)(?:a?x{2004}|b)$";
    let file = file_of(pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let bound = longest(&file);
    assert!(bound < 2_004, "{pattern} is matched against {bound} bytes");
    for (value, inside) in [
        ("x".repeat(2004), 0),
        ("x".repeat(bound), 0),
        ("b".to_owned(), 64),
    ] {
        let (holds, covering, _, _) = one_read(&file, &value);
        assert_eq!(covering, inside, "{} bytes under {pattern}", value.len());
        assert!(
            holds <= stated,
            "one read of 64 members of {pattern} holds {holds} bytes after one checked write of \
             {} bytes; 64 members of {COSTLIEST}, \"the costliest pattern found\" of the \
             contract ({STATED_ONE_READ} bytes stated), hold {stated} bytes on this host",
            value.len()
        );
    }
    let (pattern, value) = largest(|n| {
        (
            format!(r"^(?s:.*)[!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{{}}]{{{n}}}$"),
            "a".repeat(n),
        )
    });
    let (holds, covering, _, _) = one_read(&file_of(&pattern, 64), &value);
    assert_eq!(covering, 64, "the write is inside every grant of {pattern}");
    assert!(
        holds <= stated,
        "one read of 64 members of {pattern} holds {holds} bytes after one checked write; 64 \
         members of {COSTLIEST} hold {stated} bytes on this host"
    );
}

/// Added in fix round 2. compatibility.md § 2.1 states one rounded bound on
/// what one read and one checked write hold at any moment, the transient
/// bytes of compiling a pattern and of the check's cache included: 5 MiB.
/// The most this thread held is measured for the costliest pattern in bytes
/// held and for the one with the highest peak found, a leading any-run
/// before a class of 47 byte ranges. The figures are printed for the record.
#[test]
fn one_read_and_one_checked_write_peak_under_the_stated_bound() {
    let any_run = largest(|n| {
        (
            format!(r"^(?s:.*)[!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{{}}]{{{n}}}$"),
            "a".repeat(n),
        )
    });
    for (pattern, value) in [(COSTLIEST.to_owned(), costliest_value()), any_run] {
        let file = file_of(&pattern, 64);
        assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
        let before = live();
        PEAK.with(|peak| peak.set(before));
        let (holds, covering, _, _) = one_read(&file, &value);
        let peak = PEAK.with(Cell::get) - before;
        assert_eq!(covering, 64, "the write is inside every grant of {pattern}");
        println!(
            "one read and one checked write of 64 members of {pattern}: {holds} bytes held \
             after it, {peak} at the most"
        );
        assert!(
            holds <= peak && peak <= STATED_BOUND,
            "one read and one checked write of 64 members of {pattern} hold {peak} bytes at the \
             most ({holds} after the check); the contract states no more than {STATED_BOUND}"
        );
    }
}

/// RED on the tree under review. Invariant 6 of the security review, time:
/// its case (`one_read_of_a_file_within_the_stated_limits_takes_under_a_second`)
/// times the read of the file and leaves out the check of the write, which
/// the host does at every keyed write too. A file within every stated limit
/// whose 64 patterns keep about 2,000 states of the PikeVM alive on every
/// byte (the lazy DFA gives up under its 65,536-byte cache), checked against
/// a value of 3,000 bytes, well inside the request limit of 64 KiB: the
/// check takes seconds of this thread's CPU time, and the time grows with
/// the value's length (measured in the test profile: 1.3 s to 3.1 s for
/// 2,007 bytes, 7.2 s for 4,000 bytes).
///
/// Changed in fix round 2 (the work bound): the reader matches a pattern
/// member against no value longer than its `max_value_bytes`, which is under
/// 3,000 bytes for this pattern, so the write is inside no grant (it
/// asserted 64 covering grants). The bound on the time stays, and holds for
/// the value of exactly `max_value_bytes` too, and for a pattern of the same
/// family whose longest value matches it (`^(?s:.*)a?x{150}$`: all 64
/// cover).
#[test]
fn one_checked_write_within_the_stated_limits_takes_under_a_second() {
    let pattern = "^(?s:.*)a?x{2007}$";
    let file = file_of(pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let bound = longest(&file);
    assert!(bound < 3_000, "{pattern} is matched against {bound} bytes");
    for value in ["x".repeat(3_000), "x".repeat(bound)] {
        let (_, covering, read, checked) = one_read(&file, &value);
        assert_eq!(covering, 0, "{} bytes of x under {pattern}", value.len());
        assert!(
            checked < Duration::from_secs(1),
            "the check of one write of {} bytes against 64 members of {pattern} takes \
             {checked:?} of CPU time (the read of the file took {read:?})",
            value.len()
        );
    }
    let pattern = "^(?s:.*)a?x{150}$";
    let file = file_of(pattern, 64);
    let value = "x".repeat(longest(&file));
    let (_, covering, read, checked) = one_read(&file, &value);
    assert_eq!(covering, 64, "the write is inside every grant");
    assert!(
        checked < Duration::from_secs(1),
        "the check of one write of {} bytes against 64 members of {pattern} takes {checked:?} of \
         CPU time (the read of the file took {read:?})",
        value.len()
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
