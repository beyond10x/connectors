use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::Path;
use std::time::Duration;

use connectors_config::B10xIntegrationConfig;

use super::{is_ontology_operation, B10xIntegrationError};

pub(super) fn validate_module_socket(socket: &Path) -> Result<(), B10xIntegrationError> {
    if !socket.is_absolute() {
        return Err(B10xIntegrationError::InvalidConfiguration);
    }
    let parent = socket
        .parent()
        .ok_or(B10xIntegrationError::InvalidConfiguration)?;
    let parent_metadata = std::fs::symlink_metadata(parent)
        .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
    let socket_metadata = std::fs::symlink_metadata(socket)
        .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
    let owner = rustix::process::geteuid().as_raw();
    if !parent_metadata.is_dir()
        || parent_metadata.file_type().is_symlink()
        || parent_metadata.uid() != owner
        || parent_metadata.permissions().mode() & 0o077 != 0
        || !socket_metadata.file_type().is_socket()
        || socket_metadata.file_type().is_symlink()
        || socket_metadata.uid() != owner
        || socket_metadata.permissions().mode() & 0o077 != 0
    {
        return Err(B10xIntegrationError::InvalidConfiguration);
    }
    Ok(())
}

pub(super) fn http_client_on_socket(
    connect_timeout: Duration,
    total_timeout: Duration,
    socket: &Path,
) -> Result<reqwest::Client, B10xIntegrationError> {
    if connect_timeout.is_zero()
        || total_timeout.is_zero()
        || connect_timeout > total_timeout
        || !socket.is_absolute()
    {
        return Err(B10xIntegrationError::InvalidConfiguration);
    }
    validate_module_socket(socket)?;
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect_timeout)
        .timeout(total_timeout)
        .unix_socket(socket)
        .build()
        .map_err(|_| B10xIntegrationError::HttpClient)
}

pub(super) fn module_id(canonical: &str) -> Option<&'static str> {
    if is_ontology_operation(canonical) {
        Some("ontology")
    } else if canonical.starts_with("workspaces-") || canonical.starts_with("workspace-") {
        Some("workspaces")
    } else if canonical.starts_with("colab-") {
        Some("colab")
    } else if canonical.starts_with("work-") {
        Some("work")
    } else if canonical.starts_with("planner-") {
        Some("planner")
    } else {
        None
    }
}

pub(super) fn module_origin(
    config: &B10xIntegrationConfig,
    canonical: &str,
) -> Option<String> {
    let module = module_id(canonical)?;
    let origin = match module {
        "ontology" => config.ontology_origin(),
        "work" => config.work_origin(),
        "planner" => config.planner_origin(),
        "workspaces" => config.workspaces_origin(),
        "colab" => config.colab_origin(),
        _ => None,
    };
    origin.or_else(|| {
        config
            .module_socket(module)
            .map(|_| "http://localhost".to_owned())
    })
}

pub(super) fn module_client(
    config: &B10xIntegrationConfig,
    network_client: &reqwest::Client,
    module: &str,
    connect_timeout: Duration,
    total_timeout: Duration,
) -> Result<reqwest::Client, B10xIntegrationError> {
    config.module_socket(module).map_or_else(
        || Ok(network_client.clone()),
        |socket| http_client_on_socket(connect_timeout, total_timeout, socket),
    )
}
