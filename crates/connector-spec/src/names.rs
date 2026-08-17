//! Wire names and caller-facing symbol names are **not** the same namespace, and this module is
//! the seam between them.
//!
//! A vendor is free to name a query parameter `time.start`; babelforce does exactly that. A
//! caller-facing symbol is not: it is ASCII alphanumerics and `_` only, because it is the name a
//! caller — a model above all — addresses the parameter by, and the predecessor's engine could not
//! spell anything else in a declared name. The spelling constraint outlived the engine on purpose:
//! every shipped document already advertises these symbols, and renaming a symbol that has
//! travelled breaks every caller holding it.
//!
//! So each parameter carries two names: the **wire name**, which is what the vendor sees in the
//! query string, and the **symbol**, which is what the canonical document's contract declares and
//! a caller supplies. Nothing is mangled silently — the mapping is total, deterministic, and only
//! ever changes the symbol side.
//!
//! # Ownership (S-001)
//!
//! The predecessor allocated symbols in its emitter's `names` module and `connector-resolve`
//! reproduced the allocation because the document did not carry it. This module is the allocation's
//! one home now: `catalog-build` runs it at build time and **stores the symbol in the document**,
//! so every consumer reads data and nothing reproduces the rule.

use std::collections::BTreeSet;

/// Symbols the request assembly binds for its own use. A vendor parameter that would map onto one
/// of these is disambiguated instead of being allowed to shadow it — a parameter named `url` must
/// not quietly overwrite the URL the request is built from.
///
/// The list is reserved unconditionally, not per operation: a `GET` binds neither `payload` nor
/// `content_type`, but reserving them everywhere keeps a parameter's symbol independent of which
/// other parameters its operation happens to declare, so adding a body field later cannot rename a
/// symbol that already travelled.
///
/// `form_sep` is the same story one story later (predecessor C-144): only an all-optional form
/// body binds it, but it is reserved everywhere so that declaring a form encoding on an operation
/// can never rename that operation's other symbols.
///
/// **This list is frozen with the shipped catalogue.** Every committed document allocated around
/// exactly these six names; growing or shrinking the list would shift symbols that already
/// travelled.
const RESERVED: &[&str] = &[
    "base",
    "url",
    "content_type",
    "payload",
    "form_sep",
    "response",
];

/// A parameter name the allocator refuses, with the operation it is about and the reason.
#[derive(Debug, thiserror::Error)]
#[error("operation `{operation}`'s parameter `{name}` cannot carry a symbol: {reason}")]
pub struct NameError {
    /// The operation whose parameter is refused.
    pub operation: String,
    /// The offending wire name.
    pub name: String,
    /// Why it is refused.
    pub reason: &'static str,
}

/// Hands out a unique caller-facing symbol for each vendor parameter name, in declaration order.
pub struct Symbols {
    taken: BTreeSet<String>,
}

impl Default for Symbols {
    fn default() -> Self {
        Self::new()
    }
}

impl Symbols {
    /// A fresh allocator, pre-seeded with the request assembly's own [`RESERVED`] symbols.
    pub fn new() -> Self {
        Self {
            taken: RESERVED.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// The caller-facing symbol for `wire`, reserving it so no later parameter can collide with it.
    ///
    /// The mapping, in order:
    ///
    /// 1. every character outside `[A-Za-z0-9_]` becomes `_` (`time.start` → `time_start`);
    /// 2. a name that would start with a digit is prefixed with `p_`, because a leading digit is
    ///    not a spellable identifier start;
    /// 3. a name already taken — by [`RESERVED`] or by an earlier parameter that normalized to the
    ///    same thing — gains a `_2`, `_3`, … suffix.
    ///
    /// Step 3 is why this is a stateful allocator rather than a pure function: `time.start` and
    /// `time_start` are distinct wire names that normalize identically, and collapsing them would
    /// send one parameter's value under the other's name.
    pub fn allocate(&mut self, operation: &str, wire: &str) -> Result<String, NameError> {
        if wire.is_empty() {
            return Err(NameError {
                operation: operation.to_string(),
                name: wire.to_string(),
                reason: "it is empty",
            });
        }
        // A brace would be read as an interpolation delimiter by the template the URL is built
        // from, corrupting the query string rather than escaping it.
        if wire.contains('{') || wire.contains('}') {
            return Err(NameError {
                operation: operation.to_string(),
                name: wire.to_string(),
                reason: "it contains a brace, which would corrupt the URL template",
            });
        }

        let mut symbol: String = wire
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        if symbol.starts_with(|c: char| c.is_ascii_digit()) {
            symbol.insert_str(0, "p_");
        }

        if self.taken.contains(&symbol) {
            let base = symbol.clone();
            let mut n = 2;
            while self.taken.contains(&symbol) {
                symbol = format!("{base}_{n}");
                n += 1;
            }
        }

        self.taken.insert(symbol.clone());
        Ok(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identifier_safe_name_is_left_alone() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("op", "per_page").unwrap(), "per_page");
        assert_eq!(symbols.allocate("op", "agentId").unwrap(), "agentId");
    }

    /// The babelforce case: dots are not identifier-safe, and a dotted symbol would have reparsed
    /// as field access under the predecessor's engine — the spelling rule the shipped documents
    /// froze.
    #[test]
    fn a_dotted_vendor_name_becomes_an_identifier() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("op", "time.start").unwrap(), "time_start");
    }

    #[test]
    fn two_names_that_normalize_alike_stay_distinct() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("op", "time.start").unwrap(), "time_start");
        assert_eq!(
            symbols.allocate("op", "time_start").unwrap(),
            "time_start_2"
        );
    }

    /// The request assembly's own symbols are not up for grabs — a vendor parameter called `url`
    /// must not overwrite the URL the request is built from.
    #[test]
    fn the_reserved_symbols_are_not_handed_out() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("op", "url").unwrap(), "url_2");
        assert_eq!(symbols.allocate("op", "base").unwrap(), "base_2");
        assert_eq!(symbols.allocate("op", "payload").unwrap(), "payload_2");
        assert_eq!(symbols.allocate("op", "response").unwrap(), "response_2");
        assert_eq!(
            symbols.allocate("op", "content_type").unwrap(),
            "content_type_2"
        );
    }

    #[test]
    fn a_leading_digit_is_prefixed() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("op", "2fa").unwrap(), "p_2fa");
    }

    #[test]
    fn an_empty_or_brace_bearing_name_is_refused() {
        let mut symbols = Symbols::new();
        assert!(symbols.allocate("op", "").is_err());
        assert!(symbols.allocate("op", "a{b}").is_err());
    }
}
