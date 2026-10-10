//! `contexts.list`, profile `kubernetes-kubeconfig-contexts`: the contexts of
//! the kubeconfig a local connection was configured with. The typed model is
//! `connectors_kubernetes.contexts` in the adapter's ESS specification.
//!
//! Only the composition reads the file, at the path its configuration fixed;
//! this module turns those bytes into names. The deserialized shape below names
//! exactly the fields the projection keeps, so users, cluster servers,
//! certificate authorities, keys, tokens, exec plugins and auth providers are
//! skipped by the parser and never become values here.
use connectors_core::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The largest kubeconfig this binding reads.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
/// The most contexts one file may carry; a larger file refuses rather than
/// being listed in part.
pub const MAX_CONTEXTS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Context {
    pub name: String,
    pub cluster: String,
    pub namespace: Option<String>,
    pub current: bool,
}

#[derive(Deserialize)]
struct File {
    #[serde(default, rename = "current-context")]
    current_context: Option<String>,
    #[serde(default)]
    contexts: Option<Vec<Named>>,
}
#[derive(Deserialize)]
struct Named {
    name: String,
    context: Entry,
}
#[derive(Deserialize)]
struct Entry {
    cluster: String,
    #[serde(default)]
    namespace: Option<String>,
}

fn refused() -> Error {
    Error::new(
        ErrorCode::InvalidInput,
        "configured kubeconfig is not a kubeconfig this binding can read",
    )
}

/// The contexts of one kubeconfig, in file order. A file that is not a
/// kubeconfig, has more than [`MAX_CONTEXTS`] contexts, repeats a context name
/// or has a name outside its bound refuses as a whole: a partial or ambiguous
/// list would misstate which context a name selects.
pub fn contexts(bytes: &[u8]) -> Result<Vec<Context>> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(refused());
    }
    let file: File = serde_yaml_ng::from_slice(bytes).map_err(|_| refused())?;
    let named = file.contexts.unwrap_or_default();
    if named.len() > MAX_CONTEXTS {
        return Err(refused());
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(named.len());
    for Named { name, context } in named {
        let namespace = context.namespace.filter(|namespace| !namespace.is_empty());
        if !(1..=256).contains(&name.len())
            || !(1..=256).contains(&context.cluster.len())
            || namespace.as_ref().is_some_and(|n| n.len() > 63)
            || !seen.insert(name.clone())
        {
            return Err(refused());
        }
        out.push(Context {
            current: file.current_context.as_deref() == Some(name.as_str()),
            name,
            cluster: context.cluster,
            namespace,
        });
    }
    Ok(out)
}
