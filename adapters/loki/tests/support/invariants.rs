//! The `x-ess-invariants` of the schemas the pinned `ess` generates from
//! `spec/ess`, evaluated against a document, so a test can hold the executable to
//! the whole model and not only to its JSON Schema. Shared by `ess_model.rs` and
//! `adversary_pass1.rs`.
//!
//! The grammar is the rendered predicate form: `and`, `or`, `not`, parentheses,
//! comparisons (`==`, `!=`, `>=`, `<=`, `>`, `<`), `starts_with`, `contains`,
//! `defined(path)`, `exists x in list: …`, `forall x in list: …` and
//! `distinct x in list by x.member`. A path ending in `.count` is a text's character
//! count or a list's length. A bare word on the right is the fact it names when one
//! exists, else an enum variant. As in ESS, a comparison over an absent optional
//! fact holds, and so does a quantifier over an absent list. Any other form panics,
//! so a new invariant cannot pass unchecked.
#![allow(dead_code)]
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Open,
    Close,
    Colon,
    Text(String),
    Word(String),
    Operator(&'static str),
}

fn tokens(source: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' => i += 1,
            '(' => {
                out.push(Token::Open);
                i += 1;
            }
            ')' => {
                out.push(Token::Close);
                i += 1;
            }
            ':' => {
                out.push(Token::Colon);
                i += 1;
            }
            '"' => {
                let start = i;
                i += 1;
                while chars[i] != '"' {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
                let literal: String = chars[start..i].iter().collect();
                out.push(Token::Text(serde_json::from_str(&literal).unwrap()));
            }
            '=' | '!' | '>' | '<' => {
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                let operator = ["==", "!=", ">=", "<="]
                    .into_iter()
                    .find(|o| *o == two)
                    .or_else(|| [">", "<"].into_iter().find(|o| o.starts_with(c)))
                    .unwrap_or_else(|| panic!("unrecognised operator in {source}"));
                i += operator.len();
                out.push(Token::Operator(operator));
            }
            _ => {
                let start = i;
                while i < chars.len() && !" ():".contains(chars[i]) && !"=!<>".contains(chars[i]) {
                    i += 1;
                }
                out.push(Token::Word(chars[start..i].iter().collect()));
            }
        }
    }
    out
}

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    source: &'a str,
}

#[derive(Debug)]
enum Expression {
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Not(Box<Expression>),
    Defined(String),
    Compare(String, &'static str, Operand),
    StartsWith(String, String),
    Contains(String, String),
    Exists(String, String, Box<Expression>),
    Forall(String, String, Box<Expression>),
    Distinct(String, String, String),
}

#[derive(Debug)]
enum Operand {
    Literal(Value),
    Word(String),
}

impl Parser<'_> {
    fn fail(&self) -> ! {
        panic!("unrecognised invariant form: {}", self.source)
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }
    fn next(&mut self) -> Token {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .unwrap_or_else(|| self.fail());
        self.at += 1;
        token
    }
    fn word(&mut self) -> String {
        match self.next() {
            Token::Word(w) => w,
            _ => self.fail(),
        }
    }
    fn expect(&mut self, token: Token) {
        if self.next() != token {
            self.fail()
        }
    }
    fn keyword(&mut self, keyword: &str) {
        if self.word() != keyword {
            self.fail()
        }
    }
    fn or(&mut self) -> Expression {
        let mut parts = vec![self.and()];
        while self.peek() == Some(&Token::Word("or".into())) {
            self.at += 1;
            parts.push(self.and());
        }
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            Expression::Or(parts)
        }
    }
    fn and(&mut self) -> Expression {
        let mut parts = vec![self.unary()];
        while self.peek() == Some(&Token::Word("and".into())) {
            self.at += 1;
            parts.push(self.unary());
        }
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            Expression::And(parts)
        }
    }
    fn unary(&mut self) -> Expression {
        match self.peek() {
            Some(Token::Open) => {
                self.at += 1;
                let inner = self.or();
                self.expect(Token::Close);
                inner
            }
            Some(Token::Word(w)) if w == "not" => {
                self.at += 1;
                Expression::Not(Box::new(self.unary()))
            }
            Some(Token::Word(w)) if w == "defined" => {
                self.at += 1;
                self.expect(Token::Open);
                let path = self.word();
                self.expect(Token::Close);
                Expression::Defined(path)
            }
            Some(Token::Word(w)) if w == "exists" || w == "forall" => {
                let quantifier = w.clone();
                self.at += 1;
                let name = self.word();
                self.keyword("in");
                let list = self.word();
                self.expect(Token::Colon);
                let body = Box::new(self.unary());
                if quantifier == "exists" {
                    Expression::Exists(name, list, body)
                } else {
                    Expression::Forall(name, list, body)
                }
            }
            Some(Token::Word(w)) if w == "distinct" => {
                self.at += 1;
                let name = self.word();
                self.keyword("in");
                let list = self.word();
                self.keyword("by");
                Expression::Distinct(name, list, self.word())
            }
            Some(Token::Word(_)) => {
                let left = self.word();
                match self.next() {
                    Token::Operator(operator) => {
                        let right = match self.next() {
                            Token::Text(t) => Operand::Literal(Value::String(t)),
                            Token::Word(w) => match serde_json::from_str::<Value>(&w) {
                                Ok(literal @ (Value::Number(_) | Value::Bool(_))) => {
                                    Operand::Literal(literal)
                                }
                                _ => Operand::Word(w),
                            },
                            _ => self.fail(),
                        };
                        Expression::Compare(left, operator, right)
                    }
                    Token::Word(w) if w == "starts_with" || w == "contains" => {
                        let Token::Text(text) = self.next() else {
                            self.fail()
                        };
                        if w == "starts_with" {
                            Expression::StartsWith(left, text)
                        } else {
                            Expression::Contains(left, text)
                        }
                    }
                    _ => self.fail(),
                }
            }
            _ => self.fail(),
        }
    }
}

fn parse(source: &str) -> Expression {
    let mut parser = Parser {
        tokens: tokens(source),
        at: 0,
        source,
    };
    let expression = parser.or();
    if parser.at != parser.tokens.len() {
        parser.fail()
    }
    expression
}

/// The value a path names: a bound variable or a field of the document, then
/// members; a final `.count` is a character count or a length.
fn resolve(document: &Value, bound: &BTreeMap<String, Value>, path: &str) -> Option<Value> {
    let (path, count) = match path.strip_suffix(".count") {
        Some(path) => (path, true),
        None => (path, false),
    };
    let mut segments = path.split('.');
    let first = segments.next()?;
    let mut value = bound.get(first).or_else(|| document.get(first))?.clone();
    for segment in segments {
        value = value.get(segment)?.clone();
    }
    if !count {
        return Some(value);
    }
    match value {
        Value::String(s) => Some(json!(s.chars().count())),
        Value::Array(a) => Some(json!(a.len())),
        other => panic!("count of {other}"),
    }
}

fn evaluate(document: &Value, bound: &BTreeMap<String, Value>, expression: &Expression) -> bool {
    let list = |path: &str| -> Option<Vec<Value>> {
        match resolve(document, bound, path)? {
            Value::Array(items) => Some(items),
            other => panic!("{path} is not a list: {other}"),
        }
    };
    let with = |name: &str, item: &Value| {
        let mut bound = bound.clone();
        bound.insert(name.to_owned(), item.clone());
        bound
    };
    match expression {
        Expression::And(parts) => parts.iter().all(|p| evaluate(document, bound, p)),
        Expression::Or(parts) => parts.iter().any(|p| evaluate(document, bound, p)),
        Expression::Not(inner) => !evaluate(document, bound, inner),
        Expression::Defined(path) => resolve(document, bound, path).is_some(),
        Expression::StartsWith(path, prefix) => resolve(document, bound, path)
            .is_none_or(|v| v.as_str().is_some_and(|s| s.starts_with(prefix.as_str()))),
        Expression::Contains(path, needle) => resolve(document, bound, path)
            .is_none_or(|v| v.as_str().is_some_and(|s| s.contains(needle.as_str()))),
        Expression::Compare(left, operator, right) => {
            let Some(actual) = resolve(document, bound, left) else {
                return true;
            };
            let expected = match right {
                Operand::Literal(value) => value.clone(),
                Operand::Word(word) => {
                    resolve(document, bound, word).unwrap_or_else(|| Value::String(word.clone()))
                }
            };
            match *operator {
                "==" => actual == expected,
                "!=" => actual != expected,
                ordering => {
                    let (a, e) = (actual.as_f64().unwrap(), expected.as_f64().unwrap());
                    match ordering {
                        ">=" => a >= e,
                        "<=" => a <= e,
                        ">" => a > e,
                        _ => a < e,
                    }
                }
            }
        }
        Expression::Exists(name, path, body) => list(path).is_none_or(|items| {
            items
                .iter()
                .any(|i| evaluate(document, &with(name, i), body))
        }),
        Expression::Forall(name, path, body) => list(path).is_none_or(|items| {
            items
                .iter()
                .all(|i| evaluate(document, &with(name, i), body))
        }),
        Expression::Distinct(name, path, by) => list(path).is_none_or(|items| {
            let keys: Vec<Value> = items
                .iter()
                .map(|i| resolve(document, &with(name, i), by).unwrap_or(Value::Null))
                .collect();
            keys.iter()
                .enumerate()
                .all(|(index, key)| !keys[..index].contains(key))
        }),
    }
}

/// Whether one rendered invariant holds of `document`.
pub fn holds(document: &Value, invariant: &str) -> bool {
    evaluate(document, &BTreeMap::new(), &parse(invariant.trim()))
}

/// Every invariant and alphabet of every definition `document` reaches from
/// definition `name`: through properties, list items, map values and the payload
/// of a union's variant.
pub fn check(schema: &Value, name: &str, document: &Value) -> Result<(), String> {
    let definition = &schema["$defs"][name];
    if definition["x-ess-kind"] == "union" {
        let tag = definition["x-ess-union-tag"].as_str().unwrap();
        let variant = definition["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["properties"][tag]["const"] == document[tag])
            .ok_or_else(|| format!("{name}: no variant {}", document[tag]))?;
        return match (
            variant["properties"]["value"]["$ref"].as_str(),
            document.get("value"),
        ) {
            (Some(reference), Some(value)) => {
                check(schema, reference.trim_start_matches("#/$defs/"), value)
            }
            _ => Ok(()),
        };
    }
    // A newtype's invariants read `value`, the representation it wraps.
    let subject = if definition["x-ess-kind"] == "newtype" {
        json!({ "value": document })
    } else {
        document.clone()
    };
    for invariant in definition["x-ess-invariants"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let invariant = invariant.as_str().unwrap();
        if !holds(&subject, invariant) {
            return Err(format!("{name}: {invariant}"));
        }
    }
    // `alphabet:` reaches the schema only as this annotation, which no JSON
    // Schema validator enforces.
    if let Some(alphabet) = definition["x-ess-alphabet"].as_str() {
        let text = document
            .as_str()
            .ok_or_else(|| format!("{name}: not text"))?;
        if let Some(c) = text.chars().find(|c| !alphabet.contains(*c)) {
            return Err(format!("{name}: {c:?} is outside its alphabet"));
        }
    }
    let reference = |schema: &Value| {
        schema["$ref"]
            .as_str()
            .map(|r| r.trim_start_matches("#/$defs/").to_owned())
    };
    for (key, property) in definition["properties"].as_object().into_iter().flatten() {
        let Some(value) = document.get(key) else {
            continue;
        };
        if let Some(target) = reference(property) {
            check(schema, &target, value)?;
        }
        if let (Some(target), Some(items)) = (reference(&property["items"]), value.as_array()) {
            for item in items {
                check(schema, &target, item)?;
            }
        }
        if let (Some(target), Some(entries)) = (
            reference(&property["additionalProperties"]),
            value.as_object(),
        ) {
            for entry in entries.values() {
                check(schema, &target, entry)?;
            }
        }
    }
    Ok(())
}

/// The JSON Schema and every invariant of the model's root type.
pub fn conforms(schema: &Value, document: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let root = schema["$ref"]
        .as_str()
        .unwrap()
        .trim_start_matches("#/$defs/");
    check(schema, root, document)
}
