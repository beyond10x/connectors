//! Cached descriptions and durable suppression owned by local configuration.
use super::*;
use crate::local::{Failure as HostFailure, metadata::Metadata};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};

/// The release version of this build, recorded with every bootstrap it
/// remembers (`LocalRuntimeRecord.build_version`, `ess/domains/cli.yaml`).
pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The cached bootstrap column: the bootstrap's own members and, beside them,
/// `build_version`, the version of the build that wrote it. A record written
/// before the version was recorded carries none.
#[derive(Serialize)]
struct Recorded<'a> {
    #[serde(flatten)]
    bootstrap: &'a Bootstrap,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_version: Option<&'a str>,
}

pub(crate) fn encode_cached(
    bootstrap: &Bootstrap,
    build_version: Option<&str>,
) -> std::result::Result<String, HostFailure> {
    serde_json::to_string(&Recorded {
        bootstrap,
        build_version,
    })
    .map_err(db_error)
}

/// A cached bootstrap column's bootstrap, unvalidated, and its build version.
pub(crate) fn decode_cached(
    text: &str,
) -> std::result::Result<(Bootstrap, Option<String>), HostFailure> {
    let mut value: serde_json::Value =
        connectors_core::read_json(text.as_bytes()).map_err(db_error)?;
    let build_version = match value
        .as_object_mut()
        .ok_or(HostFailure::MetadataUnavailable)?
        .remove("build_version")
    {
        None => None,
        Some(serde_json::Value::String(version)) if !version.is_empty() => Some(version),
        Some(_) => return Err(HostFailure::MetadataUnavailable),
    };
    let bootstrap = serde_json::from_value(value).map_err(db_error)?;
    Ok((bootstrap, build_version))
}

pub struct State {
    path: PathBuf,
}
impl State {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
    pub fn prepare(&self) -> std::result::Result<(), HostFailure> {
        Metadata::update(&self.path, true).map(|_| ())
    }
    pub fn cached(
        &self,
        instance: &str,
        selection: &str,
    ) -> std::result::Result<Option<Bootstrap>, HostFailure> {
        let db = Metadata::inspect(&self.path)?;
        let value: Option<String> = db.connection.query_row("SELECT bootstrap FROM local_runtime_instances WHERE instance_id=?1 AND selection=?2", params![instance,selection], |r| r.get(0)).optional().map_err(db_error)?;
        value
            .map(|v| {
                if v.len() > INPUT_LIMIT {
                    return Err(HostFailure::MetadataUnavailable);
                }
                let (bootstrap, _) = decode_cached(&v)?;
                bootstrap
                    .validate()
                    .map_err(|_| HostFailure::MetadataUnavailable)?;
                Ok(bootstrap)
            })
            .transpose()
    }
    /// The version of the build that wrote `instance`'s cached bootstrap; none
    /// without a bootstrap or for one written before the version was recorded.
    pub fn build_version(
        &self,
        instance: &str,
    ) -> std::result::Result<Option<String>, HostFailure> {
        let db = Metadata::inspect(&self.path)?;
        let value: Option<String> = db
            .connection
            .query_row(
                "SELECT bootstrap FROM local_runtime_instances WHERE instance_id=?1 AND bootstrap IS NOT NULL",
                [instance],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        Ok(value
            .map(|v| decode_cached(&v))
            .transpose()?
            .and_then(|(_, version)| version))
    }
    pub fn suppressed(&self, instance: &str) -> std::result::Result<bool, HostFailure> {
        let db = Metadata::inspect(&self.path)?;
        db.connection
            .query_row(
                "SELECT suppressed FROM local_runtime_instances WHERE instance_id=?1",
                [instance],
                |r| r.get(0),
            )
            .optional()
            .map(|v| v.unwrap_or(false))
            .map_err(db_error)
    }
    pub fn suppress(&self, instance: &str, value: bool) -> std::result::Result<(), HostFailure> {
        self.update(instance, |tx| {
            tx.execute("INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES (?1,?2) ON CONFLICT(instance_id) DO UPDATE SET suppressed=excluded.suppressed",params![instance,value]).map_err(db_error)?;
            Ok(())
        })
    }
    pub fn remember(
        &self,
        selection: &str,
        bootstrap: &Bootstrap,
    ) -> std::result::Result<(), HostFailure> {
        self.remember_before_persist(selection, bootstrap, || {})
    }

    fn remember_before_persist(
        &self,
        selection: &str,
        bootstrap: &Bootstrap,
        before_persist: impl FnOnce(),
    ) -> std::result::Result<(), HostFailure> {
        bootstrap
            .validate()
            .map_err(|_| HostFailure::MetadataUnavailable)?;
        let bytes = encode_cached(bootstrap, Some(BUILD_VERSION))?;
        if bytes.len() > INPUT_LIMIT {
            return Err(HostFailure::MetadataUnavailable);
        }
        let now = i64::try_from(connectors_sdk::now_ms()).map_err(db_error)?;
        self.update_before_persist(&bootstrap.instance, |tx| {
            tx.execute("INSERT INTO local_runtime_instances (instance_id,selection,bootstrap,observed_at_ms) VALUES (?1,?2,?3,?4) ON CONFLICT(instance_id) DO UPDATE SET selection=excluded.selection,bootstrap=excluded.bootstrap,observed_at_ms=excluded.observed_at_ms",params![bootstrap.instance,selection,bytes,now]).map_err(db_error)?;
            Ok(())
        }, before_persist)
    }
    fn update(
        &self,
        instance: &str,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> std::result::Result<(), HostFailure>,
    ) -> std::result::Result<(), HostFailure> {
        self.update_before_persist(instance, action, || {})
    }

    fn update_before_persist(
        &self,
        instance: &str,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> std::result::Result<(), HostFailure>,
        before_persist: impl FnOnce(),
    ) -> std::result::Result<(), HostFailure> {
        if !connectors_core::valid_id(instance) {
            return Err(HostFailure::InvalidConfiguration);
        }
        let mut db = Metadata::update(&self.path, true)?;
        let tx = db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let total: i64 = tx
            .query_row(
                "SELECT count(*) FROM local_runtime_instances WHERE instance_id<>?1",
                [instance],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if total >= 1000 {
            return Err(HostFailure::MetadataUnavailable);
        }
        action(&tx)?;
        tx.commit().map_err(|_| HostFailure::OutcomeUnknown)?;
        before_persist();
        db.persist_runtime_state()
    }
}
fn db_error(_: impl Sized) -> HostFailure {
    HostFailure::MetadataUnavailable
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remembered_runtime_state_survives_an_interleaved_registry_clock_advance() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        crate::local::filesystem::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let state = State::new(&path);
        state.prepare().unwrap();
        let mut observation = Metadata::update_observation(&path).unwrap();
        let descriptor = json!({
            "version":"v1alpha1", "instance":"fixture", "adapter":"adapter",
            "revision":"descriptor", "configuration_schema":{"type":"object"},
            "operations":[{"id":"read", "description":"Fixture read", "contract":"fixture/1",
                "profile":"pat", "input_schema":{"type":"object"},
                "output_schema":{"type":"object"}}]
        });
        let bootstrap: Bootstrap = serde_json::from_value(json!({
            "instance":"fixture", "adapter":"adapter", "protocol":"v1alpha1",
            "configuration_revision":"cfg", "provider_authority":"fixture-authority",
            "descriptor":descriptor.to_string(),
            "profiles":[{"id":"pat", "revision":"1", "purpose":"delegated_user",
                "subject":"user", "scheme":"http_bearer", "capability":"http-bearer",
                "minimum_scopes":[], "evidence_lifetime_ms":60000,
                "fields":[{"name":"token", "label":"Fictional credential", "max_bytes":100}]}],
            "requirements":[{"operation":"read", "profile":"pat", "scopes":[], "effect":"read"}]
        }))
        .unwrap();
        bootstrap.validate().unwrap();
        // The column holds the bootstrap and, beside it, the version of the build that wrote it.
        let bytes = encode_cached(&bootstrap, Some(BUILD_VERSION)).unwrap();

        // The observer opened before the runtime write and released the
        // physical lifecycle lock, so it cannot append while the writer holds
        // it. Remembering this exact bootstrap must not invent a dependency on
        // registry time either way.
        state
            .remember_before_persist("selection", &bootstrap, || {
                observation
                    .connection
                    .execute("UPDATE registry_clock SET last_seen_ms=1", [])
                    .unwrap();
                assert_eq!(
                    observation.persist(),
                    Err(crate::local::Failure::MetadataUnavailable)
                );
            })
            .unwrap();
        drop(observation);
        let restored = State::new(&path)
            .cached("fixture", "selection")
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(&bootstrap).unwrap()
        );
        let reopened = Metadata::inspect(&path).unwrap();
        let (saved, floor): (String, i64) = reopened
            .connection
            .query_row(
                "SELECT (SELECT bootstrap FROM local_runtime_instances WHERE instance_id='fixture'),last_seen_ms FROM registry_clock WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(saved, bytes);
        assert_eq!(floor, 0);
    }

    fn fixture_bootstrap() -> Bootstrap {
        let descriptor = json!({
            "version":"v1alpha1", "instance":"fixture", "adapter":"adapter",
            "revision":"descriptor", "configuration_schema":{"type":"object"},
            "operations":[{"id":"read", "description":"Fixture read", "contract":"fixture/1",
                "profile":"pat", "input_schema":{"type":"object"},
                "output_schema":{"type":"object"}}]
        });
        let bootstrap: Bootstrap = serde_json::from_value(json!({
            "instance":"fixture", "adapter":"adapter", "protocol":"v1alpha1",
            "configuration_revision":"cfg", "provider_authority":"fixture-authority",
            "descriptor":descriptor.to_string(),
            "profiles":[{"id":"pat", "revision":"1", "purpose":"delegated_user",
                "subject":"user", "scheme":"http_bearer", "capability":"http-bearer",
                "minimum_scopes":[], "evidence_lifetime_ms":60000,
                "fields":[{"name":"token", "label":"Fictional credential", "max_bytes":100}]}],
            "requirements":[{"operation":"read", "profile":"pat", "scopes":[], "effect":"read"}]
        }))
        .unwrap();
        bootstrap.validate().unwrap();
        bootstrap
    }

    /// story:invoke-timeout-after-dispatch-stage: the runtime record names the
    /// version of the build that wrote its bootstrap, and a reopened store
    /// reads it back beside the unchanged cached bootstrap.
    #[test]
    fn a_remembered_bootstrap_records_the_writing_builds_version() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        crate::local::filesystem::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let bootstrap = fixture_bootstrap();
        let state = State::new(&path);
        state.remember("selection", &bootstrap).unwrap();
        state.suppress("fixture", true).unwrap();
        drop(state);

        let reopened = State::new(&path);
        assert_eq!(
            reopened.build_version("fixture").unwrap().as_deref(),
            Some(env!("CARGO_PKG_VERSION")),
            "the runtime record does not name the build that wrote it"
        );
        assert_eq!(
            serde_json::to_value(reopened.cached("fixture", "selection").unwrap().unwrap())
                .unwrap(),
            serde_json::to_value(&bootstrap).unwrap()
        );
        assert!(reopened.suppressed("fixture").unwrap());
        assert_eq!(reopened.build_version("absent").unwrap(), None);
    }

    /// A store whose runtime record was written without a build version
    /// still opens: the record reads back with none, its cached bootstrap is
    /// unchanged, and the next bootstrap written records this build.
    #[test]
    fn a_runtime_record_written_without_a_build_version_still_opens() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        crate::local::metadata::legacy_fixture(&path, 3);
        let bootstrap = fixture_bootstrap();
        let bytes = serde_json::to_string(&bootstrap).unwrap();
        rusqlite::Connection::open(path.join("metadata.sqlite3"))
            .unwrap()
            .execute(
                "INSERT INTO local_runtime_instances VALUES ('fixture',0,'selection',?1,44)",
                [&bytes],
            )
            .unwrap();
        let state = State::new(&path);
        state.prepare().unwrap();
        assert_eq!(state.build_version("fixture").unwrap(), None);
        assert_eq!(
            serde_json::to_value(state.cached("fixture", "selection").unwrap().unwrap()).unwrap(),
            serde_json::to_value(&bootstrap).unwrap()
        );
        state.suppress("fixture", true).unwrap();
        assert_eq!(State::new(&path).build_version("fixture").unwrap(), None);

        state.remember("selection", &bootstrap).unwrap();
        assert_eq!(
            State::new(&path)
                .build_version("fixture")
                .unwrap()
                .as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }
}
