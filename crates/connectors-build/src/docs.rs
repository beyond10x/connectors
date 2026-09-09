//! Curated public documentation inputs; ESS owns model projection.
use super::Result;
use connectors_spec::v2::{hash, json_bytes};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    pages: Vec<Page>,
    models: Vec<Model>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    route: String,
    title: String,
    source: String,
    owner: String,
    status: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    source: String,
    route: String,
    owner: String,
}

fn public_source(root: &Path, source: &str, owner: &str) -> Result<PathBuf> {
    let prefix = if owner == "shared" {
        "contracts/".to_owned()
    } else {
        format!("adapters/{owner}/contracts/")
    };
    if !source.starts_with(&prefix)
        || source.contains('\\')
        || Path::new(source).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unpublished source: {source}").into());
    }
    let path = root.join(source).canonicalize()?;
    let allowed = root.join(prefix).canonicalize()?;
    if !path.starts_with(&allowed) || !allowed.starts_with(root) || !path.is_file() {
        return Err(format!("source escapes its owner: {source}").into());
    }
    Ok(path)
}

fn route(value: &str, routes: &mut BTreeSet<String>) -> Result<()> {
    if !value.starts_with('/')
        || value.contains("..")
        || value.contains(['?', '#', '\\'])
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/-_".contains(c))
        || !routes.insert(value.to_owned())
    {
        return Err(format!("invalid or duplicate public route: {value}").into());
    }
    Ok(())
}

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn rewrite_link(
    root: &Path,
    source: &Path,
    dest: &str,
    links: &BTreeMap<PathBuf, String>,
) -> Option<String> {
    if dest.starts_with('#') {
        return Some(dest.to_owned());
    }
    if dest.starts_with("https://") || dest.starts_with("http://") {
        return Some(dest.to_owned());
    }
    if dest.contains(':') || dest.starts_with("//") || dest.contains('\\') {
        return None;
    }
    let (file, anchor) = dest
        .split_once('#')
        .map_or((dest, None), |(a, b)| (a, Some(b)));
    let path = source.parent()?.join(file).canonicalize().ok()?;
    if !path.starts_with(root) {
        return None;
    }
    links.get(&path).map(|route| match anchor {
        Some(a) => format!("{route}#{a}"),
        None => route.clone(),
    })
}

fn safe_markdown(
    root: &Path,
    source: &Path,
    markdown: &str,
    links: &BTreeMap<PathBuf, String>,
) -> Result<(String, Vec<Value>, Vec<String>)> {
    // These are source-history sections, not normative rules. Keep all other text.
    let mut filtered = String::new();
    let mut history = false;
    for line in markdown.lines() {
        if line.starts_with("## ") {
            history = line.contains("Old evidence and disposition");
        }
        if !history {
            filtered.push_str(line);
            filtered.push('\n');
        }
    }
    let private =
        regex::Regex::new(r"(?:/home/|(?:\.\./)*\.local/|(?:\.\./)*\.engineering/)[^\s`<>\)\]]+")?;
    // Keep destinations intact until link resolution; private labels/code are refused below.
    let events: Vec<_> = Parser::new_ext(
        &filtered,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    )
    .collect();
    let mut rendered = Vec::new();
    let mut headings = Vec::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut link_stack = Vec::new();
    let mut public_links = Vec::new();
    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let title: String = events
                    .iter()
                    .skip(i + 1)
                    .take_while(|e| !matches!(e, Event::End(TagEnd::Heading(_))))
                    .filter_map(|e| match e {
                        Event::Text(t) | Event::Code(t) => Some(t.as_ref()),
                        _ => None,
                    })
                    .collect();
                let base = slug(&title);
                let count = counts.entry(base.clone()).or_default();
                let id = if *count == 0 {
                    base
                } else {
                    format!("{base}-{count}")
                };
                *count += 1;
                // The reference page supplies its own h1; preserve the source anchor and text.
                let level = if *level == pulldown_cmark::HeadingLevel::H1 {
                    pulldown_cmark::HeadingLevel::H2
                } else {
                    *level
                };
                headings.push(json!({"id":id,"title":title,"level":level as u8}));
                rendered.push(Event::Start(Tag::Heading {
                    level,
                    id: Some(id.into()),
                    classes: vec![],
                    attrs: vec![],
                }));
            }
            Event::End(TagEnd::Heading(pulldown_cmark::HeadingLevel::H1)) => {
                rendered.push(Event::End(TagEnd::Heading(
                    pulldown_cmark::HeadingLevel::H2,
                )));
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let target = rewrite_link(root, source, dest_url, links);
                link_stack.push(target.is_some());
                if let Some(target) = target {
                    public_links.push(target.clone());
                    rendered.push(Event::Start(Tag::Link {
                        link_type: *link_type,
                        dest_url: target.into(),
                        title: title.clone(),
                        id: id.clone(),
                    }));
                } else {
                    rendered.push(Event::Html(CowStr::Borrowed("<span class=\"source-note\" title=\"Source reference is not published in this documentation\">")));
                }
            }
            Event::End(TagEnd::Link) => {
                if link_stack.pop() == Some(true) {
                    rendered.push(event.clone());
                } else {
                    rendered.push(Event::Html(CowStr::Borrowed("</span>")));
                }
            }
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {}
            Event::Html(t) | Event::InlineHtml(t) => rendered.push(Event::Text(t.clone())),
            Event::Text(t) => rendered.push(Event::Text(
                private
                    .replace_all(t, "[internal source reference]")
                    .into_owned()
                    .into(),
            )),
            Event::Code(t) => rendered.push(Event::Code(
                private
                    .replace_all(t, "[internal source reference]")
                    .into_owned()
                    .into(),
            )),
            _ => rendered.push(event.clone()),
        }
    }
    let mut output = String::new();
    html::push_html(&mut output, rendered.into_iter());
    if private.is_match(&output) {
        return Err(format!("private source reference remains in {}", source.display()).into());
    }
    Ok((output, headings, public_links))
}

fn check_doc_ir(value: &Value) -> Result<()> {
    match value {
        Value::Object(fields) => {
            if let Some(block) = fields.get("block").and_then(Value::as_str)
                && !["section", "prose", "table", "list", "diagram", "code"].contains(&block)
            {
                return Err(format!("unsupported ess-docs block: {block}").into());
            }
            if let Some(inline) = fields.get("inline").and_then(Value::as_str)
                && !["text", "code", "strong", "emphasis", "link", "break"].contains(&inline)
            {
                return Err(format!("unsupported ess-docs inline: {inline}").into());
            }
            for child in fields.values() {
                check_doc_ir(child)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                check_doc_ir(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn run(root: &Path, ess: &Path, check: bool) -> Result<()> {
    let inventory: Inventory =
        serde_json::from_slice(&fs::read(root.join("website/publication.json"))?)?;
    let mut routes = BTreeSet::new();
    // Authored landing pages own these routes; generated sources cannot shadow them.
    for reserved in ["/contracts", "/adapters", "/introduction"] {
        routes.insert(reserved.to_owned());
    }
    let mut links = BTreeMap::new();
    for page in &inventory.pages {
        route(&page.route, &mut routes)?;
        let path = public_source(root, &page.source, &page.owner)?;
        if links.insert(path, page.route.clone()).is_some() {
            return Err(format!("source listed twice: {}", page.source).into());
        }
    }
    let mut pages = Vec::new();
    for page in &inventory.pages {
        let source = public_source(root, &page.source, &page.owner)?;
        let bytes = fs::read(&source)?;
        let (content, headings, public_links) =
            safe_markdown(root, &source, std::str::from_utf8(&bytes)?, &links)?;
        pages.push(json!({"kind":"contract","route":page.route,"title":page.title,"source":page.source,"owner":page.owner,"status":page.status,"digest":hash(&bytes),"html":content,"headings":headings,"links":public_links}));
    }
    let temp = tempfile::Builder::new()
        .prefix("website-reference-")
        .tempdir_in(root.join(".local/tmp"))?;
    for (index, model) in inventory.models.iter().enumerate() {
        let expected = if model.owner == "shared" {
            "ess".to_owned()
        } else {
            format!("adapters/{}/spec/ess", model.owner)
        };
        if model.source != expected {
            return Err("model root disagrees with declared owner".into());
        }
        let source = root.join(&model.source).canonicalize()?;
        if !source.starts_with(root) {
            return Err("model root escapes repository".into());
        }
        // ESS may read every file beneath this root; reject nested source symlinks too.
        super::tree(&source)?;
        let output = temp.path().join(index.to_string());
        super::run(
            Command::new(ess)
                .args(["generate", "--path"])
                .arg(&source)
                .args(["--kind", "docs-ir", "--out"])
                .arg(&output),
        )?;
        let document: Value =
            serde_json::from_slice(&fs::read(output.join("docs-ir/document.json"))?)?;
        if document["format"] != "ess-docs/1" {
            return Err("unsupported ESS documentation format".into());
        }
        check_doc_ir(&document)?;
        let model_pages = document["pages"].as_array().ok_or("missing ESS pages")?;
        let model_routes: BTreeMap<_, _> = model_pages
            .iter()
            .map(|page| {
                let id = page["id"].as_str().unwrap_or("");
                (id.to_owned(), format!("{}/{}", model.route, id))
            })
            .collect();
        for page in model_pages {
            let id = page["id"].as_str().ok_or("missing ESS page id")?;
            let page_route = model_routes.get(id).ok_or("missing model route")?;
            route(page_route, &mut routes)?;
            let title = page["title"]
                .as_array()
                .ok_or("missing ESS title")?
                .iter()
                .filter_map(|v| v["text"].as_str())
                .collect::<String>();
            pages.push(json!({"kind":"model","route":page_route,"title":title,"source":model.source,"owner":model.owner,"status":"Typed specification; runtime obligations remain","document":page,"modelRoutes":model_routes}));
        }
    }
    let output = json_bytes(&json!({"pages":pages,"ess":connectors_spec::toolchain::pin()?.ess}))?;
    let text = std::str::from_utf8(&output)?;
    if ["/home/", ".local/", ".engineering/"]
        .iter()
        .any(|needle| text.contains(needle))
    {
        return Err("private paths in public reference output".into());
    }
    let path = root.join("website/.cache/reference.json");
    if check {
        if fs::read(&path)? != output {
            return Err("website reference drift; run connectors-build docs".into());
        }
    } else {
        fs::create_dir_all(path.parent().ok_or("no output parent")?)?;
        fs::write(&path, output)?;
    }
    println!(
        "website: {} contract pages, {} total reference pages; {}",
        inventory.pages.len(),
        pages.len(),
        if check { "no drift" } else { "generated" }
    );
    Ok(())
}

/// Assemble a disposable example model without modifying the canonical domains.
pub fn examples(root: &Path, ess: &Path) -> Result<()> {
    check_walkthrough_fixtures(root)?;
    fn copy_changed(from: &Path, to: &Path) -> Result<()> {
        if from.is_dir() {
            fs::create_dir_all(to)?;
            let mut entries = fs::read_dir(from)?.collect::<std::result::Result<Vec<_>, _>>()?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                copy_changed(&entry.path(), &to.join(entry.file_name()))?;
            }
        } else {
            let bytes = fs::read(from)?;
            if fs::read(to).ok().as_deref() != Some(bytes.as_slice()) {
                fs::write(to, bytes)?;
            }
        }
        Ok(())
    }
    fs::create_dir_all(root.join(".local/tmp"))?;
    let temp = tempfile::Builder::new()
        .prefix("website-examples-")
        .tempdir_in(root.join(".local/tmp"))?;
    let model = temp.path().join("model");
    copy_changed(&root.join("ess"), &model)?;
    copy_changed(
        &root.join("website/examples/components.yaml"),
        &model.join("components.yaml"),
    )?;
    let before = super::run(
        Command::new(ess)
            .args(["specify", "validate", "--path"])
            .arg(&model),
    )?;
    print!("{}", String::from_utf8_lossy(&before.stdout));
    let dest = root.join("website/.cache/demo");
    fs::create_dir_all(&dest)?;
    for target in ["rust", "web"] {
        let generated = temp
            .path()
            .join("generated")
            .join(target)
            .join("connectors");
        super::run(
            Command::new(ess)
                .args(["generate", "synthesize", "--path"])
                .arg(&model)
                .args(["--target", target, "--out"])
                .arg(&generated),
        )?;
        copy_changed(
            &generated,
            &dest.join("generated").join(target).join("connectors"),
        )?;
    }
    // Compiler diagnostics in WASM must not disclose workstation paths.
    let mut rust_flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_else(|_| {
        std::env::var("RUSTFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("\u{1f}")
    });
    for (source, replacement) in [
        (std::env::var_os("HOME").map(PathBuf::from), "/source/user"),
        (Some(root.to_path_buf()), "/source/connectors"),
    ] {
        if let Some(source) = source {
            if !rust_flags.is_empty() {
                rust_flags.push('\u{1f}');
            }
            rust_flags.push_str(&format!(
                "--remap-path-prefix={}={replacement}",
                source.display()
            ));
        }
    }
    // One project-owned cache root, no shared target directory across worktrees.
    let output = super::run(
        Command::new("cargo")
            .current_dir(root)
            .env("CARGO_BUILD_JOBS", "2")
            .env("CARGO_ENCODED_RUSTFLAGS", rust_flags)
            .args([
                "build",
                "--locked",
                "--offline",
                "--release",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                "browser",
                "--manifest-path",
            ])
            .arg(root.join("website/examples/realization/Cargo.toml"))
            .arg("--target-dir")
            .arg(dest.join("target")),
    )?;
    print!("{}", String::from_utf8_lossy(&output.stderr));
    let wasm = dest.join("target/wasm32-unknown-unknown/release/connectors_contract_examples.wasm");
    let published = root.join("website/static/examples");
    fs::create_dir_all(&published)?;
    copy_changed(&wasm, &published.join("contracts.wasm"))?;
    let catalog = fs::read(dest.join("generated/web/connectors/catalog.json"))?;
    fs::write(published.join("catalog.json"), catalog)?;
    let sources = super::tree(&root.join("ess"))?;
    let manifest = json!({"ess":connectors_spec::toolchain::pin()?.ess,"wasm_digest":hash(&fs::read(wasm)?),"source_files":sources.iter().map(|(name,bytes)|(name.clone(),hash(bytes))).collect::<BTreeMap<_,_>>(),"composition_digest":hash(&fs::read(root.join("website/examples/components.yaml"))?),"scope":"In-memory example behavior; no provider I/O or production persistence"});
    fs::write(published.join("manifest.json"), json_bytes(&manifest)?)?;
    audit(&published)?;
    println!("website: Rust/WASM contract examples built");
    Ok(())
}

fn check_walkthrough_fixtures(root: &Path) -> Result<()> {
    let fixture: Value = serde_json::from_slice(&fs::read(
        root.join("website/examples/walkthrough-fixtures.json"),
    )?)?;
    let descriptor: Value = serde_json::from_slice(&fs::read(
        root.join("adapters/gitlab/generated/descriptor.json"),
    )?)?;
    validate_walkthrough_fixtures(&descriptor, &fixture)?;
    println!("website: walkthrough input/result match the GitLab descriptor");
    Ok(())
}

fn validate_walkthrough_fixtures(descriptor: &Value, fixture: &Value) -> Result<()> {
    let operation = descriptor["operations"]
        .as_array()
        .ok_or("missing descriptor operations")?
        .iter()
        .find(|op| op["id"] == fixture["operation"])
        .ok_or("walkthrough operation is not in the GitLab descriptor")?;
    for (name, schema) in [("input", "input_schema"), ("result", "output_schema")] {
        let validator = jsonschema::validator_for(&operation[schema])?;
        if !validator.is_valid(&fixture[name]) {
            return Err(format!("walkthrough {name} does not match the GitLab descriptor").into());
        }
    }
    Ok(())
}

/// Inspect bytes as well as text: WASM can embed diagnostic source paths.
pub fn audit(directory: &Path) -> Result<()> {
    let files = super::tree(directory)?;
    if files.is_empty() {
        return Err("public output is empty".into());
    }
    for (name, bytes) in &files {
        for forbidden in [
            "/home/",
            "/Users/",
            ".local/",
            ".engineering/",
            "docs/evidence/",
        ] {
            if bytes
                .windows(forbidden.len())
                .any(|window| window == forbidden.as_bytes())
            {
                return Err(
                    format!("private path marker {forbidden:?} in public output {name}").into(),
                );
            }
        }
    }
    println!(
        "website: {} public files checked; no private path markers",
        files.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_title_keeps_anchor_and_text_without_a_second_h1() {
        let (html, headings, _) = safe_markdown(
            Path::new("/repo"),
            Path::new("/repo/contracts/a.md"),
            "# A contract\n\n## Rules\nPreserve this guarantee.",
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(html.contains("<h2 id=\"a-contract\">A contract</h2>"));
        assert!(!html.contains("<h1"));
        assert!(html.contains("Preserve this guarantee."));
        assert_eq!(headings[0]["id"], "a-contract");
        assert_eq!(headings[0]["level"], 2);
        assert_eq!(headings[1]["id"], "rules");
    }
    #[test]
    fn walkthrough_fixtures_follow_the_real_descriptor() {
        let descriptor: Value = serde_json::from_str(include_str!(
            "../../../adapters/gitlab/generated/descriptor.json"
        ))
        .unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../website/examples/walkthrough-fixtures.json"
        ))
        .unwrap();
        validate_walkthrough_fixtures(&descriptor, &fixture).unwrap();
        let mut invalid = fixture.clone();
        invalid["input"]["state"] = json!("opened");
        assert!(
            validate_walkthrough_fixtures(&descriptor, &invalid).is_err(),
            "the operation does not support an open-issues filter"
        );
        invalid = fixture.clone();
        invalid["result"]["provenance"]
            .as_object_mut()
            .unwrap()
            .remove("source_revision");
        assert!(validate_walkthrough_fixtures(&descriptor, &invalid).is_err());
        invalid = fixture;
        invalid["operation"] = json!("invented.operation");
        assert!(validate_walkthrough_fixtures(&descriptor, &invalid).is_err());
    }
    #[test]
    fn public_audit_checks_embedded_binary_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("example.wasm"),
            b"\0asm\0/home/example/private.rs",
        )
        .unwrap();
        assert!(audit(dir.path()).is_err());
        fs::write(
            dir.path().join("example.wasm"),
            b"\0asm\0/source/example.rs",
        )
        .unwrap();
        audit(dir.path()).unwrap();
    }
    #[test]
    fn rejects_duplicate_and_escaping_routes() {
        let mut set = BTreeSet::new();
        route("/contracts/auth", &mut set).unwrap();
        assert!(route("/contracts/auth", &mut set).is_err());
        assert!(route("/contracts/../secret", &mut set).is_err());
    }
    #[test]
    fn imported_markdown_cannot_execute_html_or_script_links() {
        let root = Path::new("/repo");
        let (html,_,_)=safe_markdown(root,&root.join("contracts/a.md"),"# Rules\n<script>alert(1)</script>\n\n[x](javascript:alert)\n\n`/home/example/private`",&BTreeMap::new()).unwrap();
        assert!(!html.contains("<script>"));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("/home/"));
        assert!(html.contains("&lt;script&gt;"));
    }
    #[test]
    fn preserves_rules_and_stable_heading_anchors() {
        let (html,headings,_)=safe_markdown(Path::new("/repo"),Path::new("/repo/contracts/a.md"),"## 2. Old evidence and disposition\nHistorical source\n## 3. Rules\nNever resend.\n## 3. Rules\nStill applies.",&BTreeMap::new()).unwrap();
        assert!(!html.contains("Historical source"));
        assert!(html.contains("Never resend."));
        assert_eq!(headings[0]["id"], "3-rules");
        assert_eq!(headings[1]["id"], "3-rules-1");
    }
    #[test]
    fn refuses_unknown_projection_nodes() {
        assert!(check_doc_ir(&json!({"block":"hidden-new-rule"})).is_err());
        assert!(check_doc_ir(&json!({"inline":"unhandled"})).is_err());
    }
    #[test]
    fn resolves_only_selected_local_links() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("contracts")).unwrap();
        let a = dir.path().join("contracts/a.md");
        let b = dir.path().join("contracts/b.md");
        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();
        let links = BTreeMap::from([(b.canonicalize().unwrap(), "/contracts/b".to_owned())]);
        assert_eq!(
            rewrite_link(dir.path(), &a, "b.md#rules", &links),
            Some("/contracts/b#rules".to_owned())
        );
        assert!(rewrite_link(dir.path(), &a, "a.md", &links).is_none());
        assert!(rewrite_link(dir.path(), &a, "//example.test/pixel", &links).is_none());
    }
    #[test]
    fn cannot_publish_a_source_outside_its_owner() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("contracts")).unwrap();
        fs::write(dir.path().join("private.md"), "private").unwrap();
        assert!(public_source(dir.path(), "contracts/../private.md", "shared").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                dir.path().join("private.md"),
                dir.path().join("contracts/link.md"),
            )
            .unwrap();
            assert!(public_source(dir.path(), "contracts/link.md", "shared").is_err());
        }
    }
}
