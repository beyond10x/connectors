//! Shaping a result (story:invoke-shapes-its-result; `contracts/cli/v1alpha1/semantics.md`,
//! "Shaping a result"): `--fields` names the members of a provider result to print and `--each`
//! the array whose elements are printed one per line.
//!
//! A pure function over `serde_json::Value`: the paths are parsed once ([`Fields::parse`],
//! [`Path::parse`]) and applied to a result, which yields the shaped value and the paths that
//! matched nothing. Nothing here reads, writes or calls anything, so the same paths shape an
//! `invoke` answer, an MCP tool result and each record of a walk.
//!
//! The grammar is a path list, not a filter language (decision D240 a = A):
//!
//! - `name(.name)*` names a member, `[]` maps over an array (`events[].field_name`);
//! - `alias:path` prints the value the path reaches under `alias`;
//! - a path beginning with `/` is an RFC 6901 pointer whose tokens are member names, for a key
//!   holding `.`, `[` or `:` (`/page[after]`; `~1` is `/` and `~0` is `~`);
//! - paths are separated by commas.
use serde_json::{Map, Value, json};

/// The most paths one `--fields` names.
pub const MAX_PATHS: usize = 256;
/// The most steps one path takes.
pub const MAX_STEPS: usize = 64;
/// The members a shaped `invoke` answer keeps whatever the fields name, so that a write's
/// outcome cannot be projected away.
pub const KEPT: [&str; 3] = ["request_id", "mutation", "source_audit"];
/// The output names no field may take: [`KEPT`], and `missing` and `records`, which the answer
/// adds itself.
pub const RESERVED: [&str; 5] = [
    "missing",
    "mutation",
    "records",
    "request_id",
    "source_audit",
];

/// A path list or path the grammar refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Member(String),
    Each,
}

/// One member path without `[]`: the array `--each` prints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    text: String,
    steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Field {
    /// The path as the caller wrote it, alias included: what `missing` lists.
    text: String,
    alias: Option<String>,
    steps: Vec<Step>,
}

/// A parsed `--fields` path list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fields(Vec<Field>);

/// The members a path list selected from one value, and the paths that matched nothing, in
/// the order given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shaped {
    pub value: Map<String, Value>,
    pub missing: Vec<String>,
}

/// A shaped `invoke` answer: the records `--each` prints one per line, then the result the
/// envelope carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub records: Vec<Value>,
    pub result: Value,
}

/// A character a dotted name may hold: not a separator of the grammar, whitespace or a
/// control character.
fn name_char(c: char) -> bool {
    !matches!(c, '.' | '[' | ']' | ',' | ':' | '/') && !c.is_whitespace() && !c.is_control()
}

/// The steps of `text`: a pointer when it begins with `/`, else dotted names and `[]`.
fn steps(text: &str) -> Result<Vec<Step>, Malformed> {
    let steps = match text.strip_prefix('/') {
        Some(pointer) => pointer
            .split('/')
            .map(|token| unescape(token).map(Step::Member))
            .collect::<Result<Vec<_>, _>>()?,
        None => dotted(text)?,
    };
    if steps.len() > MAX_STEPS {
        return Err(Malformed);
    }
    Ok(steps)
}

/// One RFC 6901 reference token: `~1` is `/`, `~0` is `~`, any other `~` is refused.
fn unescape(token: &str) -> Result<String, Malformed> {
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(c) = chars.next() {
        match c {
            '~' => match chars.next() {
                Some('0') => out.push('~'),
                Some('1') => out.push('/'),
                _ => return Err(Malformed),
            },
            c => out.push(c),
        }
    }
    Ok(out)
}

/// `name(.name)*` with `[]` after a name or at the start: a name is required at the start
/// (unless it is `[]`) and after each `.`.
fn dotted(text: &str) -> Result<Vec<Step>, Malformed> {
    let mut steps = Vec::new();
    let mut rest = text;
    let mut need_name = !text.starts_with("[]");
    loop {
        if need_name {
            let end = rest.find(|c: char| !name_char(c)).unwrap_or(rest.len());
            if end == 0 {
                return Err(Malformed);
            }
            steps.push(Step::Member(rest[..end].to_owned()));
            rest = &rest[end..];
            need_name = false;
        }
        if rest.is_empty() {
            return Ok(steps);
        }
        if let Some(after) = rest.strip_prefix("[]") {
            steps.push(Step::Each);
            rest = after;
        } else if let Some(after) = rest.strip_prefix('.') {
            rest = after;
            need_name = true;
        } else {
            return Err(Malformed);
        }
    }
}

impl Path {
    /// One path of member names, dotted or a pointer; `[]`, an alias and a comma are refused.
    pub fn parse(text: &str) -> Result<Path, Malformed> {
        let steps = steps(text)?;
        if steps.contains(&Step::Each) {
            return Err(Malformed);
        }
        Ok(Path {
            text: text.to_owned(),
            steps,
        })
    }

    /// The value the path names in `value`.
    pub fn find<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        self.find_under(value, None)
    }

    /// The value the path names in a provider result: a path whose first name a catalog result lacks reads its `body`.
    pub fn find_in_result<'a>(&self, result: &'a Value) -> Option<&'a Value> {
        self.find_under(result, body(result))
    }

    fn find_under<'a>(&self, value: &'a Value, fallback: Option<&'a Value>) -> Option<&'a Value> {
        self.steps.iter().try_fold(
            base(&self.steps, value, fallback),
            |value, step| match step {
                Step::Member(name) => value.get(name),
                Step::Each => None,
            },
        )
    }
}

impl Fields {
    /// A comma-separated path list. Each item is trimmed; an empty item, a malformed path, an
    /// alias that is not a name, a path without an alias that begins with `[]`, an output name
    /// in [`RESERVED`], an alias that repeats another output name, more than [`MAX_PATHS`]
    /// paths or more than [`MAX_STEPS`] steps in one is refused.
    pub fn parse(text: &str) -> Result<Fields, Malformed> {
        let mut fields: Vec<Field> = Vec::new();
        for item in text.split(',') {
            let item = item.trim();
            let (alias, path) = match item.split_once(':') {
                Some((alias, path)) if !item.starts_with('/') => {
                    if alias.is_empty() || !alias.chars().all(name_char) {
                        return Err(Malformed);
                    }
                    (Some(alias.to_owned()), path)
                }
                _ => (None, item),
            };
            let steps = steps(path)?;
            let field = Field {
                text: item.to_owned(),
                alias,
                steps,
            };
            let output = field.output().ok_or(Malformed)?;
            let taken = fields.iter().any(|other| {
                other.output() == Some(output) && (field.alias.is_some() || other.alias.is_some())
            });
            if taken || RESERVED.contains(&output) {
                return Err(Malformed);
            }
            fields.push(field);
            if fields.len() > MAX_PATHS {
                return Err(Malformed);
            }
        }
        Ok(Fields(fields))
    }

    /// The members of `value` the paths name.
    pub fn apply(&self, value: &Value) -> Shaped {
        self.apply_under(value, None)
    }

    /// The members of a provider result the paths name: a path whose first name a catalog result lacks reads its `body`.
    pub fn apply_to_result(&self, result: &Value) -> Shaped {
        self.apply_under(result, body(result))
    }

    fn apply_under(&self, value: &Value, fallback: Option<&Value>) -> Shaped {
        // One tree of unaliased paths per base, so paths sharing a prefix print one member.
        let bases = [Some(value), fallback];
        let mut trees = [Node::default(), Node::default()];
        let mut aliased = Map::new();
        let mut missing = Vec::new();
        for field in &self.0 {
            let at = usize::from(falls_back(&field.steps, value, fallback));
            let Some(from) = bases[at] else { continue };
            if !reaches(from, &field.steps) {
                missing.push(field.text.clone());
                continue;
            }
            match &field.alias {
                Some(alias) => {
                    if let Some(picked) = pick(from, &field.steps) {
                        aliased.insert(alias.clone(), picked);
                    }
                }
                None => trees[at].insert(&field.steps),
            }
        }
        let mut shaped = Map::new();
        for (tree, from) in trees.iter().zip(bases) {
            if let Some(Value::Object(members)) = from.and_then(|from| tree.eval(from)) {
                shaped.extend(members);
            }
        }
        shaped.extend(aliased);
        Shaped {
            value: shaped,
            missing,
        }
    }
}

impl Field {
    /// The top-level output name: the alias, else the path's first member.
    fn output(&self) -> Option<&str> {
        match (&self.alias, self.steps.first()) {
            (Some(alias), _) => Some(alias),
            (None, Some(Step::Member(name))) => Some(name),
            _ => None,
        }
    }
}

/// The `body` of a catalog result, read by a path whose first name the result lacks: a
/// catalog result is `{status, body, provenance}` with an integer `status`.
fn body(result: &Value) -> Option<&Value> {
    result
        .get("status")
        .is_some_and(Value::is_u64)
        .then(|| result.get("body"))
        .flatten()
}

/// Whether `result` is a catalog result whose `status` is not 2xx: it is printed whole.
pub fn failed_status(result: &Value) -> bool {
    body(result).is_some()
        && !result["status"]
            .as_u64()
            .is_some_and(|status| (200..300).contains(&status))
}

/// Whether a path reads `fallback`: there is one and the path begins with a name `value` lacks.
fn falls_back(steps: &[Step], value: &Value, fallback: Option<&Value>) -> bool {
    fallback.is_some()
        && matches!(steps.first(), Some(Step::Member(name)) if value.get(name).is_none())
}

/// The value a path reads from ([`falls_back`]).
fn base<'a>(steps: &[Step], value: &'a Value, fallback: Option<&'a Value>) -> &'a Value {
    match fallback {
        Some(fallback) if falls_back(steps, value, Some(fallback)) => fallback,
        _ => value,
    }
}

/// Whether the path reaches a value: a named member exists, and `[]` meets an array that is
/// empty or has an element the rest of the path reaches.
fn reaches(value: &Value, steps: &[Step]) -> bool {
    match steps.split_first() {
        None => true,
        Some((Step::Member(name), rest)) => value.get(name).is_some_and(|v| reaches(v, rest)),
        Some((Step::Each, rest)) => value
            .as_array()
            .is_some_and(|items| items.is_empty() || items.iter().any(|item| reaches(item, rest))),
    }
}

/// The value an aliased path reaches, `[]` mapping to the elements it reaches.
fn pick(value: &Value, steps: &[Step]) -> Option<Value> {
    match steps.split_first() {
        None => Some(value.clone()),
        Some((Step::Member(name), rest)) => pick(value.get(name)?, rest),
        Some((Step::Each, rest)) => {
            let items = value.as_array()?;
            let picked: Vec<Value> = items.iter().filter_map(|item| pick(item, rest)).collect();
            (items.is_empty() || !picked.is_empty()).then_some(Value::Array(picked))
        }
    }
}

/// The unaliased paths of one base as a tree: a member printed whole where a path ends, the
/// named members of an object, the elements of an array mapped.
#[derive(Default)]
struct Node {
    whole: bool,
    members: Vec<(String, Node)>,
    each: Option<Box<Node>>,
}

impl Node {
    fn insert(&mut self, steps: &[Step]) {
        let mut node = self;
        for step in steps {
            node = match step {
                Step::Member(name) => {
                    let at = match node.members.iter().position(|(n, _)| n == name) {
                        Some(at) => at,
                        None => {
                            node.members.push((name.clone(), Node::default()));
                            node.members.len() - 1
                        }
                    };
                    &mut node.members[at].1
                }
                Step::Each => node.each.get_or_insert_with(Default::default),
            };
        }
        node.whole = true;
    }

    fn eval(&self, value: &Value) -> Option<Value> {
        if self.whole {
            return Some(value.clone());
        }
        match value {
            Value::Object(object) => {
                let members: Map<String, Value> = self
                    .members
                    .iter()
                    .filter_map(|(name, node)| Some((name.clone(), node.eval(object.get(name)?)?)))
                    .collect();
                (!members.is_empty()).then_some(Value::Object(members))
            }
            Value::Array(items) => {
                let node = self.each.as_ref()?;
                let mapped: Vec<Value> = items.iter().filter_map(|item| node.eval(item)).collect();
                (items.is_empty() || !mapped.is_empty()).then_some(Value::Array(mapped))
            }
            _ => None,
        }
    }
}

/// `answer`, an `InvokeResult`, shaped by `fields` and `each` (`connectors.cli.InvokeShaped`);
/// `None` when it is printed whole: neither is given, it carries no `result`, or the result is
/// a catalog result whose status is not 2xx.
///
/// With `fields` alone the result holds the members they name. With `each` it holds `records`,
/// the number of elements of the array `each` names, each element printed as a record and
/// shaped by `fields` when given; a field is missing when no element has it, and `each` when
/// it names no array. Either way the members in [`KEPT`] are copied from the answer when it has
/// them, and `missing` lists the paths that matched nothing, when any did.
pub fn answer(answer: &Value, fields: Option<&Fields>, each: Option<&Path>) -> Option<Answer> {
    if fields.is_none() && each.is_none() {
        return None;
    }
    let result = answer.get("result")?;
    if failed_status(result) {
        return None;
    }
    let (records, mut shaped, missing) = match (each, fields) {
        (Some(each), fields) => {
            let mut records = Vec::new();
            let mut missing = Vec::new();
            match each.find_in_result(result).and_then(Value::as_array) {
                Some(items) => {
                    // A field is missing when every element misses it.
                    let mut everywhere: Option<Vec<String>> = None;
                    for item in items {
                        match fields {
                            Some(fields) => {
                                let shaped = fields.apply(item);
                                everywhere = Some(match everywhere {
                                    None => shaped.missing,
                                    Some(before) => before
                                        .into_iter()
                                        .filter(|path| shaped.missing.contains(path))
                                        .collect(),
                                });
                                records.push(Value::Object(shaped.value));
                            }
                            None => records.push(item.clone()),
                        }
                    }
                    missing.extend(everywhere.unwrap_or_default());
                }
                None => missing.push(each.text.clone()),
            }
            let count = records.len();
            (
                records,
                Map::from_iter([("records".into(), json!(count))]),
                missing,
            )
        }
        (None, Some(fields)) => {
            let shaped = fields.apply_to_result(result);
            (Vec::new(), shaped.value, shaped.missing)
        }
        (None, None) => return None,
    };
    for kept in KEPT {
        if let Some(value) = answer.get(kept).filter(|value| !value.is_null()) {
            shaped.insert(kept.into(), value.clone());
        }
    }
    if !missing.is_empty() {
        shaped.insert("missing".into(), json!(missing));
    }
    Some(Answer {
        records,
        result: Value::Object(shaped),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(text: &str) -> Fields {
        Fields::parse(text).unwrap_or_else(|_| panic!("{text:?} parses"))
    }

    fn shaped(text: &str, value: &Value) -> (Value, Vec<String>) {
        let shaped = fields(text).apply(value);
        (Value::Object(shaped.value), shaped.missing)
    }

    fn member(name: &str) -> Step {
        Step::Member(name.into())
    }

    /// story:invoke-shapes-its-result: `name(.name)*`, `[]` after a name or at the start, a
    /// pointer's tokens as names with `~1` and `~0` unescaped.
    #[test]
    fn the_grammar_reads_dotted_names_each_and_pointers() {
        assert_eq!(
            dotted("ticket.id"),
            Ok(vec![member("ticket"), member("id")])
        );
        assert_eq!(
            dotted("events[].field_name"),
            Ok(vec![member("events"), Step::Each, member("field_name")])
        );
        assert_eq!(dotted("[][]"), Ok(vec![Step::Each, Step::Each]));
        assert_eq!(dotted("[].id"), Ok(vec![Step::Each, member("id")]));
        assert_eq!(
            steps("/page[after]"),
            Ok(vec![member("page[after]")]),
            "story:invoke-shapes-its-result: RFC 6901"
        );
        assert_eq!(
            steps("/meta/a~1b~0c.d"),
            Ok(vec![member("meta"), member("a/b~c.d")])
        );
        assert_eq!(steps("/"), Ok(vec![member("")]));
    }

    /// story:invoke-shapes-its-result: every malformed path is refused when the list is
    /// parsed, before anything is applied.
    #[test]
    fn a_malformed_path_list_is_refused() {
        for malformed in [
            "",
            "a,",
            ",a",
            "a..b",
            ".a",
            "a.",
            "a[",
            "a]",
            "a[]b",
            "a.[]",
            "a[0]",
            "a b",
            "a:b:c",
            ":a",
            "a.b:c",
            "x:",
            "[].id",
            "/a~2",
            "/a~",
            "missing",
            "request_id",
            "mutation.classification",
            "source_audit",
            "records",
            "records:a",
            "m:a,m:b",
            "ticket:a,ticket.id",
            "ticket.id,ticket:a",
        ] {
            assert_eq!(
                Fields::parse(malformed),
                Err(Malformed),
                "story:invoke-shapes-its-result: {malformed:?}"
            );
        }
        let deep = vec!["a"; MAX_STEPS + 1].join(".");
        assert_eq!(Fields::parse(&deep), Err(Malformed));
        let many = vec!["a"; MAX_PATHS + 1].join(",");
        assert_eq!(Fields::parse(&many), Err(Malformed));
        assert!(Fields::parse(&vec!["a"; MAX_STEPS].join(".")).is_ok());
        assert!(Fields::parse(" a , b:c.d ,/e.f,g:/h:i,j[].k,m:[].n").is_ok());
    }

    /// story:invoke-shapes-its-result: `--each` names one array by member names; `[]`, an
    /// alias or a second path is refused.
    #[test]
    fn an_each_path_is_member_names_only() {
        assert!(Path::parse("audits").is_ok());
        assert!(Path::parse("body.audits").is_ok());
        assert!(Path::parse("/page[after]").is_ok());
        for malformed in ["", "audits[]", "a[].b", "x:audits", "a,b", "a..b"] {
            assert_eq!(Path::parse(malformed), Err(Malformed), "{malformed:?}");
        }
    }

    /// story:invoke-shapes-its-result: unaliased paths print nested as written, sharing a
    /// prefix prints one member, an alias prints the value it reaches under its name.
    #[test]
    fn fields_select_nested_members_and_aliases_rename() {
        let ticket =
            json!({"ticket":{"id":101,"status":"open","subject":"s","via":{"channel":"web"}}});
        assert_eq!(
            shaped("ticket.id,state:ticket.status", &ticket),
            (json!({"ticket":{"id":101},"state":"open"}), vec![])
        );
        assert_eq!(
            shaped("ticket.id,ticket.via.channel,ticket.via", &ticket).0,
            json!({"ticket":{"id":101,"via":{"channel":"web"}}})
        );
        assert_eq!(
            shaped("ticket,ticket.id", &ticket).0,
            ticket,
            "a whole member includes its named members"
        );
        assert_eq!(
            shaped(
                "ticket.assignee_id",
                &json!({"ticket":{"assignee_id":null}})
            ),
            (json!({"ticket":{"assignee_id":null}}), vec![]),
            "a null member is present"
        );
    }

    /// story:invoke-shapes-its-result: `[]` maps over an array; an element the rest of the
    /// path does not reach is left out, an empty array stays empty, and a path no element
    /// reaches is missing.
    #[test]
    fn each_step_maps_over_an_array() {
        let audits = json!({"audits":[
            {"id":1,"events":[{"type":"Comment"},{"field_name":"status","value":"new"}]},
            {"id":2,"events":[]},
            {"id":3}]});
        assert_eq!(
            shaped("audits[].id,audits[].events[].field_name", &audits),
            (
                json!({"audits":[
                    {"id":1,"events":[{"field_name":"status"}]},
                    {"id":2,"events":[]},
                    {"id":3}]}),
                vec![]
            )
        );
        assert_eq!(
            shaped("names:audits[].events[].field_name", &audits).0,
            json!({"names":[["status"],[]]})
        );
        assert_eq!(
            shaped("audits[].author_id,audits[].id", &audits),
            (
                json!({"audits":[{"id":1},{"id":2},{"id":3}]}),
                vec!["audits[].author_id".to_owned()]
            )
        );
        assert_eq!(
            shaped("audits[].id", &json!({"audits":[]})),
            (json!({"audits":[]}), vec![])
        );
        assert_eq!(
            shaped("ids:[].id", &json!([{"id":1},{"x":2}])).0,
            json!({"ids":[1]})
        );
    }

    /// story:invoke-shapes-its-result: a path that reaches nothing is listed, as written, in
    /// the order given; a member through a scalar or an array, or `[]` over an object, reaches
    /// nothing.
    #[test]
    fn a_path_that_matches_nothing_is_missing() {
        let value = json!({"ticket":{"id":101,"tags":["a"]},"count":3});
        assert_eq!(
            shaped(
                "x:ticket.absent,ticket.id, count.value ,ticket.tags.a,ticket[].id",
                &value
            ),
            (
                json!({"ticket":{"id":101}}),
                vec![
                    "x:ticket.absent".to_owned(),
                    "count.value".to_owned(),
                    "ticket.tags.a".to_owned(),
                    "ticket[].id".to_owned()
                ]
            )
        );
    }

    /// story:invoke-shapes-its-result: a pointer reads a key holding `.` or `[`.
    #[test]
    fn a_pointer_reads_a_key_the_dotted_form_cannot_name() {
        let value = json!({"page[after]":"c2","a.b":{"c/d":1}});
        assert_eq!(
            shaped("/page[after],x:/a.b/c~1d", &value).0,
            json!({"page[after]":"c2","x":1})
        );
    }

    fn catalog(status: u64, body: Value) -> Value {
        json!({"status":status,"body":body,"provenance":{"instance":"i","resource":"/r",
            "observed_at_unix_ms":1,"source_revision":null}})
    }

    /// story:invoke-shapes-its-result: a path whose first name a catalog result lacks reads
    /// its `body`; one the result has (`status`) reads the result; `body.` may be written.
    #[test]
    fn a_catalog_result_is_read_through_its_body_when_the_name_is_not_its_own() {
        let result = catalog(200, json!({"ticket":{"id":101},"status":"body-status"}));
        let shaped = fields("ticket.id,status,b:body.status").apply_to_result(&result);
        assert_eq!(
            Value::Object(shaped.value),
            json!({"ticket":{"id":101},"status":200,"b":"body-status"})
        );
        assert!(shaped.missing.is_empty());
        let plain = json!({"ticket":{"id":1},"body":{"ticket":{"id":2}}});
        assert_eq!(
            Value::Object(fields("x.y").apply_to_result(&plain).value),
            json!({}),
            "a result without an integer status is no catalog result"
        );
        assert_eq!(
            Path::parse("ticket").unwrap().find_in_result(&result),
            Some(&json!({"id":101}))
        );
        assert_eq!(Path::parse("ticket").unwrap().find(&result), None);
    }

    /// story:invoke-shapes-its-result: a catalog result whose status is not 2xx is printed
    /// whole; neither flag prints the answer whole too.
    #[test]
    fn a_non_2xx_catalog_result_is_not_shaped() {
        let fields = fields("status");
        for status in [199, 300, 404, 500] {
            let answer = json!({"result":catalog(status, json!({"error":"x"}))});
            assert!(failed_status(&answer["result"]));
            assert_eq!(answer_of(&answer, Some(&fields), None), None, "{status}");
        }
        let ok = json!({"result":catalog(204, json!(null))});
        assert_eq!(
            answer_of(&ok, Some(&fields), None).map(|a| a.result),
            Some(json!({"status":204}))
        );
        assert_eq!(answer_of(&ok, None, None), None);
    }

    fn answer_of(answer: &Value, fields: Option<&Fields>, each: Option<&Path>) -> Option<Answer> {
        super::answer(answer, fields, each)
    }

    /// story:invoke-shapes-its-result: `--each` prints each element, shaped by the fields,
    /// and a result counting them; a field no element has, or an each path that names no
    /// array, is missing.
    #[test]
    fn each_prints_records_and_counts_them() {
        let answer = json!({"connector":"c","result":catalog(200, json!({"audits":[
            {"id":1,"author_id":7},{"id":2}],"meta":{"has_more":false}}))});
        let each = Path::parse("audits").unwrap();
        assert_eq!(
            answer_of(&answer, Some(&fields("id,author_id,absent")), Some(&each)),
            Some(Answer {
                records: vec![json!({"id":1,"author_id":7}), json!({"id":2})],
                result: json!({"records":2,"missing":["absent"]}),
            })
        );
        assert_eq!(
            answer_of(&answer, None, Some(&each)).map(|a| a.records),
            Some(vec![json!({"id":1,"author_id":7}), json!({"id":2})])
        );
        assert_eq!(
            answer_of(
                &answer,
                Some(&fields("id")),
                Some(&Path::parse("meta").unwrap())
            ),
            Some(Answer {
                records: vec![],
                result: json!({"records":0,"missing":["meta"]}),
            })
        );
        let empty = json!({"result":catalog(200, json!({"audits":[]}))});
        assert_eq!(
            answer_of(&empty, Some(&fields("absent")), Some(&each)),
            Some(Answer {
                records: vec![],
                result: json!({"records":0}),
            })
        );
    }

    /// story:invoke-shapes-its-result: `request_id`, `mutation` and `source_audit` are
    /// copied whatever the fields name.
    #[test]
    fn a_write_keeps_its_request_mutation_and_source_audit() {
        let answer = json!({"connector":"c","instance":"i","operation":"o","revision":"r",
            "result":catalog(200, json!({"ticket":{"id":1,"status":"solved"}})),
            "request_id":"req-1","mutation":{"classification":"outcome_unknown"},
            "source_audit":{"id":"a"}});
        assert_eq!(
            answer_of(&answer, Some(&fields("ticket.status")), None).map(|a| a.result),
            Some(json!({"ticket":{"status":"solved"},"request_id":"req-1",
                "mutation":{"classification":"outcome_unknown"},"source_audit":{"id":"a"}}))
        );
    }
}
