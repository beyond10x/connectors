//! Cached descriptions and durable suppression owned by local configuration.
use super::*;
use crate::local::{Failure as HostFailure, metadata::Metadata};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};

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
                let bootstrap: Bootstrap = connectors_core::read_json(v.as_bytes())
                    .map_err(|_| HostFailure::MetadataUnavailable)?;
                bootstrap
                    .validate()
                    .map_err(|_| HostFailure::MetadataUnavailable)?;
                Ok(bootstrap)
            })
            .transpose()
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
        bootstrap
            .validate()
            .map_err(|_| HostFailure::MetadataUnavailable)?;
        let bytes = serde_json::to_string(bootstrap).map_err(db_error)?;
        if bytes.len() > INPUT_LIMIT {
            return Err(HostFailure::MetadataUnavailable);
        }
        let now = i64::try_from(connectors_sdk::now_ms()).map_err(db_error)?;
        self.update(&bootstrap.instance, |tx| {
            tx.execute("INSERT INTO local_runtime_instances (instance_id,selection,bootstrap,observed_at_ms) VALUES (?1,?2,?3,?4) ON CONFLICT(instance_id) DO UPDATE SET selection=excluded.selection,bootstrap=excluded.bootstrap,observed_at_ms=excluded.observed_at_ms",params![bootstrap.instance,selection,bytes,now]).map_err(db_error)?;
            Ok(())
        })
    }
    fn update(
        &self,
        instance: &str,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> std::result::Result<(), HostFailure>,
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
        tx.commit().map_err(|_| HostFailure::OutcomeUnknown)
    }
}
fn db_error(_: impl Sized) -> HostFailure {
    HostFailure::MetadataUnavailable
}
