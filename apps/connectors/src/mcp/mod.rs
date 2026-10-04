//! Native MCP composition. Only this application knows how a native deployment
//! selection constrains the generic owner's admitted operation.
use connectors_host::local::{
    config::{Config, Paths},
    filesystem, operation_curation,
    owner::{self, Code, ReadPolicy},
    runtime,
};
use connectors_mcp::configuration::{Configuration, FILE_LIMIT};
use serde_json::json;
use std::path::PathBuf;

mod entry;
mod projection;
mod session;
pub(super) use entry::{run, selected};

pub(super) struct ProjectionPolicy;

fn companion(paths: &Paths) -> PathBuf {
    let mut path = paths.config.as_os_str().to_owned();
    path.push(".mcp-stdio.json");
    path.into()
}
fn load(paths: &Paths) -> owner::Result<Configuration> {
    let bytes = filesystem::read_bounded(filesystem::private_file(&companion(paths))?, FILE_LIMIT)?;
    Configuration::parse(&bytes).map_err(|_| Code::InvalidConfiguration.into())
}
fn revision(
    configuration: &Configuration,
    adapter: &connectors_host::local::config::Adapter,
    bootstrap: &runtime::Bootstrap,
    curation_sha256: &str,
) -> String {
    connectors_core::digest(
        &json!({"projection":configuration.document(),"adapter":adapter,"bootstrap":bootstrap,"curation":curation_sha256}),
    )
}
impl ReadPolicy for ProjectionPolicy {
    fn metadata(
        &self,
        paths: &Paths,
        alias: &str,
        adapter: &connectors_host::local::config::Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> owner::Result<owner::ProjectionMetadata> {
        let configuration = load(paths)?;
        let curation = operation_curation::load(paths, alias, adapter, bootstrap)?;
        Ok(owner::ProjectionMetadata {
            revision: revision(&configuration, adapter, bootstrap, &curation.source_sha256),
            operations: curation.operations,
        })
    }

    fn admit(
        &self,
        paths: &Paths,
        alias: &str,
        request: &owner::approval_issuance::Request<'_>,
        expected: &str,
    ) -> owner::Result<()> {
        // Repeat generic lookup admission so disappearance/revocation while the
        // earlier check was running cannot disclose a native private selection.
        let host = Config::load(&paths.config)?;
        let adapter = host.adapters.get(alias).ok_or(Code::NotGranted)?;
        if !adapter.permissions.operations.contains(request.operation) {
            return Err(Code::NotGranted.into());
        }
        let configuration = load(paths)?;
        let exposure =
            configuration.document().exposures.iter().find(|entry| {
                entry.adapter_alias == alias && entry.operation_ref == request.operation
            });
        if exposure.is_some_and(|entry| entry.connection_ref != request.connection) {
            return Err(Code::Forbidden.into());
        }
        let bootstrap = owner::cached(paths, alias)?;
        let curation = operation_curation::load(paths, alias, adapter, &bootstrap)?;
        if revision(&configuration, adapter, &bootstrap, &curation.source_sha256) != expected {
            return Err(Code::StaleDescription.into());
        }
        let exposure = exposure.ok_or(Code::NotFound)?;
        if !exposure.enabled {
            return Err(Code::Forbidden.into());
        }
        if !curation.operations.contains_key(request.operation) {
            return Err(Code::NotFound.into());
        }
        // This does not consult the selected approval source or credential
        // readiness. The existing coordinator independently admits execution.
        Ok(())
    }
}

#[cfg(test)]
mod tests;
