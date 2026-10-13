//! Platform-gated tests: the `#[test]` functions a `cfg` predicate keeps off
//! at least one supported platform. A test gated off a platform does not
//! compile there, so no test binary on that platform can list it; this source
//! scan names it instead, from the same module tree rustc loads.
//!
//! Only the platform dimension is decided: `unix`, `windows`, `target_os` and
//! `target_family`, with `test` true. Every other `cfg` atom (features,
//! `target_env`, `target_arch`, `debug_assertions`) is unknown and never by
//! itself keeps a test off a platform. Tests a macro expands to are not seen.
use crate::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use syn::{Attribute, Item, Meta, Token, punctuated::Punctuated};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Macos,
    Windows,
}
const PLATFORMS: [Platform; 3] = [Platform::Linux, Platform::Macos, Platform::Windows];

impl Platform {
    /// The platform this program was compiled for, if it is a supported one.
    pub fn current() -> Option<Self> {
        if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else if cfg!(windows) {
            Some(Self::Windows)
        } else {
            None
        }
    }
}

/// One test harness target: its package, its libtest target name and the
/// crate root rustc compiles it from.
#[derive(Clone, Debug)]
pub struct Target {
    pub package: String,
    pub target: String,
    pub root: PathBuf,
}

/// One gated test, by the exact name libtest gives it on a platform it runs on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Gated {
    pub package: String,
    pub target: String,
    pub test: String,
    pub platforms: Vec<Platform>,
}

/// Scan every target; the result is sorted by package, target and test.
pub fn scan(targets: &[Target]) -> Result<Vec<Gated>> {
    let mut gated = Vec::new();
    for target in targets {
        let mut tests = Vec::new();
        let file = parse(&target.root)?;
        let mask = platforms(&file.attrs)?;
        let directory = target
            .root
            .parent()
            .ok_or("crate root has no directory")?
            .to_path_buf();
        let place = Place {
            children: directory.clone(),
            paths: directory,
        };
        walk(&file.items, &place, &mut Vec::new(), mask, &mut tests)?;
        gated.extend(
            tests
                .into_iter()
                .filter(|(_, mask)| mask.iter().any(|on| !on))
                .map(|(test, mask)| Gated {
                    package: target.package.clone(),
                    target: target.target.clone(),
                    test,
                    platforms: PLATFORMS
                        .into_iter()
                        .zip(mask)
                        .filter_map(|(platform, on)| on.then_some(platform))
                        .collect(),
                }),
        );
    }
    gated.sort();
    Ok(gated)
}

/// Every gated test that runs on `current` must be in the compiled list of its
/// target there; one that is not means the scan read the sources wrongly.
/// `listed` answers whether a package, target and test is compiled here.
pub fn verify(
    gated: &[Gated],
    current: Platform,
    listed: impl Fn(&str, &str, &str) -> bool,
) -> Result<()> {
    let absent: Vec<String> = gated
        .iter()
        .filter(|g| g.platforms.contains(&current))
        .filter(|g| !listed(&g.package, &g.target, &g.test))
        .map(|g| format!("{}::{}::{}", g.package, g.target, g.test))
        .collect();
    if absent.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "platform-gated scan names tests this platform's test binaries do not list: {}",
            absent.join(", ")
        )
        .into())
    }
}

/// Where a module's children live: `children` for `mod x;`, `paths` for
/// `#[path = "..."] mod x;`. A crate root, a `mod.rs` and a file loaded by
/// `#[path]` keep their children beside them; `x.rs` keeps them in `x/`.
struct Place {
    children: PathBuf,
    paths: PathBuf,
}

fn parse(path: &Path) -> Result<syn::File> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?)
}

fn walk(
    items: &[Item],
    place: &Place,
    module: &mut Vec<String>,
    mask: [bool; 3],
    tests: &mut Vec<(String, [bool; 3])>,
) -> Result<()> {
    for item in items {
        match item {
            Item::Fn(function) if function.attrs.iter().any(is_test) => {
                let mut name = module.clone();
                name.push(function.sig.ident.to_string());
                tests.push((name.join("::"), and(mask, platforms(&function.attrs)?)));
            }
            Item::Mod(declaration) => {
                let name = declaration.ident.to_string();
                let mask = and(mask, platforms(&declaration.attrs)?);
                let path = declaration.attrs.iter().find_map(|attr| match &attr.meta {
                    Meta::NameValue(value) if value.path.is_ident("path") => Some(&value.value),
                    _ => None,
                });
                let path = match path {
                    Some(syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(path),
                        ..
                    })) => Some(path.value()),
                    Some(_) => return Err(format!("unsupported #[path] on module {name}").into()),
                    None => None,
                };
                module.push(name.clone());
                if let Some((_, content)) = &declaration.content {
                    let inner = place.children.join(&name);
                    let place = Place {
                        children: inner.clone(),
                        paths: inner,
                    };
                    walk(content, &place, module, mask, tests)?;
                } else {
                    let (file, place) = match path {
                        Some(path) => {
                            let file = place.paths.join(path);
                            let directory = file.parent().unwrap_or(Path::new("")).to_path_buf();
                            (
                                file,
                                Place {
                                    children: directory.clone(),
                                    paths: directory,
                                },
                            )
                        }
                        None => {
                            let flat = place.children.join(format!("{name}.rs"));
                            let nested = place.children.join(&name).join("mod.rs");
                            if flat.is_file() {
                                let place = Place {
                                    children: place.children.join(&name),
                                    paths: place.children.clone(),
                                };
                                (flat, place)
                            } else if nested.is_file() {
                                let directory = place.children.join(&name);
                                (
                                    nested,
                                    Place {
                                        children: directory.clone(),
                                        paths: directory,
                                    },
                                )
                            } else {
                                return Err(format!(
                                    "module {} has no file at {} or {}",
                                    module.join("::"),
                                    flat.display(),
                                    nested.display()
                                )
                                .into());
                            }
                        }
                    };
                    if !file.is_file() {
                        return Err(format!(
                            "module {} has no file at {}",
                            module.join("::"),
                            file.display()
                        )
                        .into());
                    }
                    let parsed = parse(&file)?;
                    let mask = and(mask, platforms(&parsed.attrs)?);
                    walk(&parsed.items, &place, module, mask, tests)?;
                }
                module.pop();
            }
            _ => {}
        }
    }
    Ok(())
}

/// `#[test]` and the attribute macros that expand to it, such as `#[tokio::test]`.
fn is_test(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
}

fn and(left: [bool; 3], right: [bool; 3]) -> [bool; 3] {
    [
        left[0] && right[0],
        left[1] && right[1],
        left[2] && right[2],
    ]
}

/// The platforms every `cfg` attribute here admits: a platform is excluded
/// only where some predicate is definitely false on it.
fn platforms(attrs: &[Attribute]) -> Result<[bool; 3]> {
    let mut mask = [true; 3];
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("cfg")) {
        let predicate: Meta = attr.parse_args()?;
        for (index, platform) in PLATFORMS.into_iter().enumerate() {
            if evaluate(&predicate, platform)? == Some(false) {
                mask[index] = false;
            }
        }
    }
    Ok(mask)
}

/// Three-valued: `None` is an atom this scan does not decide.
fn evaluate(predicate: &Meta, platform: Platform) -> Result<Option<bool>> {
    let (os, family) = match platform {
        Platform::Linux => ("linux", "unix"),
        Platform::Macos => ("macos", "unix"),
        Platform::Windows => ("windows", "windows"),
    };
    Ok(match predicate {
        Meta::Path(path) if path.is_ident("unix") => Some(family == "unix"),
        Meta::Path(path) if path.is_ident("windows") => Some(family == "windows"),
        Meta::Path(path) if path.is_ident("test") => Some(true),
        Meta::Path(_) => None,
        Meta::NameValue(pair) => {
            let value = match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(value),
                    ..
                }) => value.value(),
                _ => return Ok(None),
            };
            if pair.path.is_ident("target_os") {
                Some(value == os)
            } else if pair.path.is_ident("target_family") {
                Some(value == family)
            } else {
                None
            }
        }
        Meta::List(list) => {
            let arguments =
                list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            let values = arguments
                .iter()
                .map(|argument| evaluate(argument, platform))
                .collect::<Result<Vec<_>>>()?;
            if list.path.is_ident("all") {
                if values.contains(&Some(false)) {
                    Some(false)
                } else if values.contains(&None) {
                    None
                } else {
                    Some(true)
                }
            } else if list.path.is_ident("any") {
                if values.contains(&Some(true)) {
                    Some(true)
                } else if values.contains(&None) {
                    None
                } else {
                    Some(false)
                }
            } else if list.path.is_ident("not") {
                match values.as_slice() {
                    [value] => value.map(|value| !value),
                    _ => return Err("cfg(not(..)) takes exactly one predicate".into()),
                }
            } else {
                None
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Platform::*;

    fn runs(predicate: &str) -> Vec<Platform> {
        let attrs: Vec<Attribute> =
            syn::parse_str::<syn::ItemFn>(&format!("#[cfg({predicate})] fn f() {{}}"))
                .unwrap()
                .attrs;
        let mask = platforms(&attrs).unwrap();
        PLATFORMS
            .into_iter()
            .zip(mask)
            .filter_map(|(p, on)| on.then_some(p))
            .collect()
    }

    #[test]
    fn cfg_predicates_decide_platforms_and_leave_other_atoms_open() {
        assert_eq!(runs("unix"), [Linux, Macos]);
        assert_eq!(runs("windows"), [Windows]);
        assert_eq!(runs("not(windows)"), [Linux, Macos]);
        assert_eq!(runs("target_os = \"linux\""), [Linux]);
        assert_eq!(runs("all(test, target_os = \"macos\")"), [Macos]);
        assert_eq!(
            runs("any(target_os = \"macos\", windows)"),
            [Macos, Windows]
        );
        assert_eq!(runs("target_family = \"unix\""), [Linux, Macos]);
        assert_eq!(runs("not(target_os = \"windows\")"), [Linux, Macos]);
        assert_eq!(
            runs("all(test, target_os = \"linux\", target_env = \"gnu\")"),
            [Linux]
        );
        assert_eq!(
            runs("all(unix, any(target_os = \"linux\", test))"),
            [Linux, Macos]
        );
        assert_eq!(runs("test"), [Linux, Macos, Windows]);
        assert_eq!(runs("feature = \"schema\""), [Linux, Macos, Windows]);
        assert_eq!(runs("not(feature = \"schema\")"), [Linux, Macos, Windows]);
        assert_eq!(runs("target_os = \"ios\""), []);
    }

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    fn target(root: &Path, package: &str, target: &str, file: &str) -> Target {
        Target {
            package: package.into(),
            target: target.into(),
            root: root.join(file),
        }
    }
    fn gated(target: &str, test: &str, platforms: &[Platform]) -> Gated {
        Gated {
            package: "p".into(),
            target: target.into(),
            test: test.into(),
            platforms: platforms.into(),
        }
    }

    /// A crate tree using every module form rustc resolves: inline, `x.rs`
    /// with a child directory, `x/mod.rs`, `#[path]` at file level and inside
    /// an inline module, file-level `#![cfg]` and `#[tokio::test]`.
    fn fixture() -> (tempfile::TempDir, Vec<Target>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "src/lib.rs",
            r#"
#[cfg(unix)]
mod a;
mod m;
#[path = "elsewhere/p.rs"]
mod p;
#[cfg(test)]
mod tests {
    #[test]
    fn everywhere() {}
    #[cfg(target_os = "linux")]
    #[test]
    fn only_linux() {}
    #[cfg(not(windows))]
    #[tokio::test]
    async fn not_windows() {}
    #[cfg(windows)]
    fn helper_not_a_test() {}
    #[cfg(windows)]
    #[path = "inner.rs"]
    mod inner;
}
"#,
        );
        write(root, "src/a.rs", "mod b;\n#[test]\nfn in_a() {}\n");
        write(
            root,
            "src/a/b.rs",
            "#[test]\n#[ignore]\nfn in_b() {}\n#[cfg(windows)]\n#[test]\nfn never() {}\n",
        );
        write(
            root,
            "src/m/mod.rs",
            "#[cfg(target_os = \"macos\")]\nmod n;\n#[test]\nfn plain() {}\n",
        );
        write(root, "src/m/n.rs", "#[test]\nfn mac() {}\n");
        write(
            root,
            "src/elsewhere/p.rs",
            "#[cfg(windows)]\nmod q;\n#[test]\nfn plain() {}\n",
        );
        write(root, "src/elsewhere/q.rs", "#[test]\nfn win() {}\n");
        write(root, "src/tests/inner.rs", "#[test]\nfn win_inner() {}\n");
        write(
            root,
            "tests/whole.rs",
            "//! doc\n#![cfg(unix)]\nmod support;\n#[test]\nfn one() {}\n",
        );
        write(root, "tests/support/mod.rs", "pub fn help() {}\n");
        write(root, "tests/open.rs", "#[test]\nfn open() {}\n");
        let targets = vec![
            target(root, "p", "p", "src/lib.rs"),
            target(root, "p", "whole", "tests/whole.rs"),
            target(root, "p", "open", "tests/open.rs"),
        ];
        (dir, targets)
    }

    #[test]
    fn scan_names_every_gated_test_by_its_libtest_name() {
        let (_dir, targets) = fixture();
        assert_eq!(
            scan(&targets).unwrap(),
            [
                gated("p", "a::b::in_b", &[Linux, Macos]),
                gated("p", "a::b::never", &[]),
                gated("p", "a::in_a", &[Linux, Macos]),
                gated("p", "m::n::mac", &[Macos]),
                gated("p", "p::q::win", &[Windows]),
                gated("p", "tests::inner::win_inner", &[Windows]),
                gated("p", "tests::not_windows", &[Linux, Macos]),
                gated("p", "tests::only_linux", &[Linux]),
                gated("whole", "one", &[Linux, Macos]),
            ]
        );
    }

    #[test]
    fn a_module_file_that_cannot_be_found_refuses_the_scan() {
        let (dir, targets) = fixture();
        std::fs::remove_file(dir.path().join("src/m/n.rs")).unwrap();
        let error = scan(&targets).unwrap_err().to_string();
        assert!(error.contains("module m::n"), "{error}");
    }

    #[test]
    fn a_gated_test_this_platform_compiles_but_the_scan_misnames_is_refused() {
        let gated = [
            gated("t", "runs_here", &[Linux, Macos]),
            gated("t", "elsewhere", &[Windows]),
        ];
        let compiled = |_: &str, target: &str, test: &str| target == "t" && test == "runs_here";
        verify(&gated, Linux, compiled).unwrap();
        let error = verify(&gated, Windows, compiled).unwrap_err().to_string();
        assert!(error.contains("p::t::elsewhere"), "{error}");
        let error = verify(&gated, Linux, |_: &str, _: &str, _: &str| false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("p::t::runs_here"), "{error}");
    }

    /// This repository's own sources: each file-level gated suite is named,
    /// item-level gates in a shared suite are named, and no entry claims every
    /// platform.
    #[test]
    fn the_workspace_scan_names_the_gated_suites() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let gated = scan(&crate::ignored::test_targets(&root).unwrap()).unwrap();
        assert!(gated.iter().all(|g| g.platforms.len() < PLATFORMS.len()));
        let named = |package: &str, target: &str| {
            gated
                .iter()
                .filter(|g| g.package == package && g.target == target)
                .count()
        };
        for (package, target) in [
            ("connectors", "consumer_launch"),
            ("connectors", "failed_connect"),
            ("connectors", "failed_connect_adversary"),
            ("connectors", "owner_build_security"),
            ("connectors", "owner_idle_exit_adversary"),
            ("connectors", "socket_path_adversary"),
            ("connectors", "local_cli"),
            ("connectors-host", "http_audit_anchor"),
            ("connectors-host", "http_audit_anchor_adversary"),
            ("connectors-host", "http_mutation_ledger_adversary"),
            ("connectors-host", "local_foundation"),
            ("connectors-host", "metadata_reopen_adversary"),
            ("connectors-host", "metadata_store_previous_pin"),
            ("connectors-host", "pooled_open_scan_adversary"),
            ("connectors-host", "service"),
            ("connectors-host", "connectors_host"),
            ("connectors-custody", "connectors_custody"),
        ] {
            assert!(named(package, target) > 0, "{package} {target}");
        }
        assert!(gated.iter().any(|g| g.package == "connectors"
            && g.target == "socket_path_adversary"
            && g.platforms == [Linux]));
    }
}
