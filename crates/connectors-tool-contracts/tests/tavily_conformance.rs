//! The Tavily contract held to `tool-conformance/1` through the adapter's own `invoke`, over the
//! runner's local fake of the provider's API; the mutants the suite must catch; and the
//! connection-filled field a caller may not send.
use connectors_core::Descriptor;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{Credential, Secret};
use connectors_tavily::{READ_POST_TIMEOUT, READ_POSTS, Tavily};
use connectors_tool_contracts::{
    AdapterToolset, Declarations, Publication, publish,
    tool_contract::{
        Contract, Toolset,
        conformance::{Outcome, Report, ScenarioContext, Suite, run_with},
        mutants::each_mutant_fails,
    },
};
use schemars::Schema;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::Arc,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Assembled at runtime so no credential-shaped literal sits in the source.
fn key() -> String {
    format!("tvly-{}", "k".repeat(24))
}

struct Fixed(String);
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn descriptor() -> Descriptor {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/tavily/generated/descriptor.json")).unwrap(),
    )
    .unwrap()
}

fn declarations() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/tavily/spec/tool-profiles.json")).unwrap(),
    )
    .unwrap()
}

fn publication(declarations: &Value) -> Publication {
    publish(
        &descriptor(),
        &Declarations::from_json(&serde_json::to_vec(declarations).unwrap()).unwrap(),
    )
    .unwrap()
}

/// The committed contract, as the generator wrote it.
fn committed() -> Contract {
    Contract::read(root().join("adapters/tavily/generated/tool-contract.yaml")).unwrap()
}

/// The Tavily adapter of instance `tavily`, its API at `base`.
fn tavily(base: &str) -> Tavily {
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap()
    .with_credential(Arc::new(Fixed(key())))
    .with_read_posts(&READ_POSTS, READ_POST_TIMEOUT)
    .unwrap();
    let effective =
        json!({"format": "connectors-tavily-local/1", "instance": "tavily", "api_base": base});
    Tavily::new("tavily", &effective, Arc::new(http))
        .unwrap()
        .with_clock(|| 1_791_190_000_000)
}

fn toolset(publication: Publication, base: &str) -> AdapterToolset {
    AdapterToolset::new(publication, descriptor(), Arc::new(tavily(base))).unwrap()
}

fn base(context: &ScenarioContext<'_>) -> String {
    // A scenario without routes never reaches the provider: its input is refused first.
    format!("{}/", context.served.unwrap_or("http://127.0.0.1:9"))
}

fn suite() -> Suite {
    Suite::read_dir(root().join("adapters/tavily/spec/tool-conformance/")).unwrap()
}

fn run(make: impl Fn(&ScenarioContext<'_>) -> Box<dyn Toolset>) -> Report {
    let scratch = tempfile::tempdir().unwrap();
    run_with(&suite(), &committed(), scratch.path(), |context| {
        Ok(make(context))
    })
}

#[test]
fn tavily_passes_tool_conformance() {
    let published = publication(&declarations());
    assert_eq!(
        published.contract,
        committed(),
        "regenerate the committed contract"
    );
    let report = run(|context| Box::new(toolset(published.clone(), &base(context))));
    let text = report.to_string();
    println!("{text}");
    assert!(report.passed(), "{text}");
    assert_eq!(report.scenarios.len(), 7, "{text}");
    assert!(report.schema_mismatches.is_empty() && report.missing.is_empty());
}

/// A mutant of the toolset: the real one with one property switched off.
struct Mutant {
    inner: AdapterToolset,
    mutation: Mutation,
}

#[derive(Clone, Copy)]
enum Mutation {
    /// Answers an error name the contract does not declare.
    UndeclaredError,
    /// Drops the list of what was found from every answer.
    DropsResults,
    /// Answers success, with nothing, for a refused call.
    SwallowsRefusal,
    /// Answers the error without the code that says which.
    DropsCode,
}

impl Toolset for Mutant {
    fn contract(&self) -> &Contract {
        self.inner.contract()
    }
    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        self.inner.schemas()
    }
    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        match (self.mutation, self.inner.invoke(operation, input)) {
            (
                Mutation::UndeclaredError,
                Outcome::Error {
                    message, fields, ..
                },
            ) => Outcome::Error {
                name: "connector.Oops".into(),
                message,
                fields,
            },
            (Mutation::DropsResults, Outcome::Ok(mut output)) => {
                if let Some(map) = output.as_object_mut() {
                    map.remove("results");
                    map.remove("pages");
                }
                Outcome::Ok(output)
            }
            (Mutation::SwallowsRefusal, Outcome::Error { .. }) => Outcome::Ok(json!({})),
            (Mutation::DropsCode, Outcome::Error { name, message, .. }) => Outcome::Error {
                name,
                message,
                fields: json!({}),
            },
            (_, outcome) => outcome,
        }
    }
}

#[test]
fn every_mutant_fails_a_scenario() {
    let published = publication(&declarations());
    let reports: Vec<(&str, Report)> = [
        ("undeclared-error", Mutation::UndeclaredError),
        ("drops-results", Mutation::DropsResults),
        ("swallows-refusal", Mutation::SwallowsRefusal),
        ("drops-code", Mutation::DropsCode),
    ]
    .into_iter()
    .map(|(name, mutation)| {
        let report = run(|context| {
            Box::new(Mutant {
                inner: toolset(published.clone(), &base(context)),
                mutation,
            })
        });
        (name, report)
    })
    .collect();
    each_mutant_fails(&reports).unwrap_or_else(|survivors| {
        panic!("mutants passed every scenario: {survivors:?}");
    });
}

/// Answers one request with `body` and hands back the request's JSON body.
fn one_answer(body: &'static str) -> (String, std::thread::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut seen = Vec::new();
        let mut byte = [0u8; 1];
        while !seen.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).unwrap();
            seen.push(byte[0]);
        }
        let head = String::from_utf8(seen).unwrap();
        let length: usize = head
            .lines()
            .find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse().unwrap())
            })
            .unwrap_or(0);
        let mut request = vec![0; length];
        socket.read_exact(&mut request).unwrap();
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        serde_json::from_slice(&request).unwrap()
    });
    (format!("http://{address}/"), handle)
}

#[test]
fn omitted_field_is_refused_and_filled() {
    let mut value = declarations();
    value["operations"]["websearch.search"]["omit"] = json!(["country"]);
    let published = publication(&value);
    let (base, request) = one_answer(r#"{"query":"q","results":[]}"#);
    let toolset =
        toolset(published, &base).fill("connector-tavily.search", "country", json!("germany"));

    let refused = toolset.invoke(
        "connector-tavily.search",
        &json!({"query": "q", "max_results": 1, "country": "france"}),
    );
    assert_eq!(
        refused,
        Outcome::Error {
            name: "connector.FieldNotAllowed".into(),
            message: "country is filled by the connection".into(),
            fields: json!({"field": "country"}),
        }
    );
    let Outcome::Ok(output) = toolset.invoke(
        "connector-tavily.search",
        &json!({"query": "q", "max_results": 1}),
    ) else {
        panic!("the search failed");
    };
    assert_eq!(output["results"], json!([]));
    // Only the second call reached the provider, carrying the connection's value.
    assert_eq!(request.join().unwrap()["country"], json!("germany"));
    // The published input has no country, and the derived schema agrees.
    let (_, input, _) = toolset
        .schemas()
        .into_iter()
        .find(|(name, _, _)| name == "connector-tavily.search")
        .unwrap();
    assert!(input.as_value()["properties"].get("country").is_none());
}
