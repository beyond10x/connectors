use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ready,
    Pending,
    ReauthorizationRequired,
    Stale,
    CustodyUnavailable,
    Revoked,
}

pub struct ObservedConnection {
    pub reference: String,
    pub instance: String,
    pub profile: String,
    pub revision: String,
    pub state: State,
    pub identity: Option<ExternalIdentity>,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub stale: bool,
}
pub struct Page {
    pub connections: Vec<ObservedConnection>,
    pub next_cursor: Option<String>,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub stale: bool,
}
pub struct PageOptions<'a> {
    pub limit: u16,
    pub cursor: Option<&'a str>,
}
pub struct ObservedAcquisition {
    pub reference: String,
    pub instance: String,
    pub state: &'static str,
    pub reason: Option<String>,
    pub connection: Option<String>,
    pub expires_at_ms: u64,
    pub observed_at_ms: u64,
}

impl Registry {
    /// Resolve a retained approval target without updating the registry clock,
    /// checking custody or granting a provider call. Expired validation
    /// evidence can identify this binding; known invalid material cannot.
    /// Invocation must independently establish current readiness. Time is read
    /// only to name the refusal of a stale connection (`changed`).
    pub fn approval_target(
        &self,
        binding: &Binding,
        reference: &str,
        scopes: &BTreeSet<String>,
    ) -> Result<String> {
        if !connectors_core::valid_id(reference) {
            return Err(Failure::InvalidInput);
        }
        let mut metadata = Metadata::inspect(&self.path).map_err(super::host_failure)?;
        let tx = metadata.connection.transaction().map_err(db)?;
        let row = Self::connection(&tx, reference)?;
        visible(&row, &binding.instance_id, &binding.adapter_id)?;
        if row.state == "revoked" {
            return Err(Failure::Revoked);
        }
        // An operation-only change leaves the connection bound (D12). An
        // executable-only change names it too: the approval binds the configured
        // executable selection, and the write it admits validates the credential
        // under that executable before anything is sent (`ExecutableChanged`).
        if !matches!(
            follow(&row.binding, binding),
            Follow::Same | Follow::Operations | Follow::Executable
        ) {
            // Whether a revalidation admits this credential is the read path's
            // readiness rule, expiry included. Only this refusal looks at
            // time, and only when the binding could otherwise follow an
            // upgrade: the wall clock, never below the recorded floor so a
            // regressed clock cannot revive an expired credential. Nothing is
            // recorded; the registry clock is not advanced.
            let usable = || {
                if row.material.is_none() {
                    return Ok(false);
                }
                let floor: i64 = tx
                    .query_row(
                        "SELECT last_seen_ms FROM registry_clock WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(db)?;
                let now = connectors_sdk::now_ms().max(u64::try_from(floor).unwrap_or(0));
                Ok(matches!(
                    readiness(&tx, &row, now, true)?,
                    State::Ready | State::Pending
                ))
            };
            return Err(super::revalidation::changed(&row, binding, usable)?);
        }
        let version = row.material.as_ref().ok_or(Failure::NotReady)?;
        let (ack, deleted, invalid, retired): (bool, bool, Option<String>, bool) = tx.query_row(
            "SELECT acknowledged,deleted,invalid_reason,retirement_fence IS NOT NULL FROM registry_materials WHERE version_id=?1 AND connection_ref=?2 AND generation_id=?3",
            params![version,row.reference,row.generation],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).map_err(db)?;
        if !ack || retired {
            return Err(Failure::MetadataUnavailable);
        }
        // A Stale connection's credential was refused, as a known invalid
        // one was (`readiness`).
        if deleted || invalid.is_some() || row.kept.stale || row.identity.is_none() {
            return Err(Failure::NotReady);
        }
        let baseline = row.baseline.as_ref().ok_or(Failure::MetadataUnavailable)?;
        if (!binding.profile.minimum_scopes.is_empty() || !scopes.is_empty())
            && !baseline.granted_scopes.as_ref().is_some_and(|granted| {
                binding.profile.minimum_scopes.is_subset(granted) && scopes.is_subset(granted)
            })
        {
            return Err(Failure::InsufficientScope);
        }
        Ok(row.revision)
    }

    /// Whether a read command may start the configured adapter to cache the
    /// description of the configuration it names (contracts/cli/v1alpha1/
    /// semantics.md, "Launching for a description"): the instance holds a
    /// published connection, `reference` when one is named, that is not revoked
    /// or Stale, whose credential a read could use (neither known invalid nor
    /// expired), whose upgrade the configured revision has not refused, and
    /// that is current under `configured`, as `connections list` reports it
    /// not stale. A connection under a changed provider authority or profile
    /// authentication, or with no credential-side digest under another
    /// configuration revision, is not current. Records nothing and grants
    /// nothing; the clock is read as `approval_target` reads it.
    pub fn launch_admits<'a>(
        &self,
        instance: &str,
        adapter: &str,
        configured: impl Into<Configured<'a>>,
        reference: Option<&str>,
    ) -> Result<bool> {
        let configured = configured.into();
        let mut metadata = Metadata::inspect(&self.path).map_err(super::host_failure)?;
        let tx = metadata.connection.transaction().map_err(db)?;
        let references = match reference {
            Some(reference) => vec![reference.to_owned()],
            None => tx
                .prepare("SELECT connection_ref FROM registry_connections WHERE instance_id=?1 AND public=1 AND state!='revoked' ORDER BY connection_ref LIMIT 64")
                .map_err(db)?
                .query_map([instance], |r| r.get::<_, String>(0))
                .map_err(db)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db)?,
        };
        let floor: i64 = tx
            .query_row(
                "SELECT last_seen_ms FROM registry_clock WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        let now = connectors_sdk::now_ms().max(u64::try_from(floor).unwrap_or(0));
        for reference in references {
            let row = match Self::connection(&tx, &reference) {
                Ok(row) => row,
                Err(Failure::NotFound | Failure::InvalidInput) => continue,
                Err(other) => return Err(other),
            };
            if visible(&row, instance, adapter).is_err()
                || row.state == "revoked"
                || row.kept.stale
                || row.material.is_none()
                || row.kept.refused.as_deref() == Some(configured.configuration_revision)
                || !configured.current(&row.binding)
            {
                continue;
            }
            if matches!(
                readiness(&tx, &row, now, true)?,
                State::Ready | State::Pending
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn describe<'a>(
        &self,
        instance: &str,
        adapter: &str,
        configured: impl Into<Configured<'a>>,
        reference: &str,
        now: u64,
        custody_available: bool,
    ) -> Result<ObservedConnection> {
        let configured = configured.into();
        self.transaction_observation(now, |tx, _, now| {
            let row = Self::connection(tx, reference)?;
            visible(&row, instance, adapter)?;
            observe(tx, row, &configured, now, custody_available)
        })
    }

    pub fn acquisition_status(
        &self,
        instance: &str,
        adapter: &str,
        reference: &str,
        now: u64,
    ) -> Result<ObservedAcquisition> {
        if !connectors_core::valid_id(reference) {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, _, now| {
            let (connection, state, failure, expires): (String,String,Option<String>,u64) = tx.query_row(
                "SELECT connection_ref,state,failure,expires_at_ms FROM registry_acquisitions WHERE acquisition_ref=?1", [reference],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,read_time(r,3)?))).optional().map_err(db)?.ok_or(Failure::NotFound)?;
            let row = Self::connection(tx, &connection)?;
            if row.binding.instance_id!=instance || row.binding.adapter_id!=adapter { return Err(Failure::NotFound); }
            let (state, reason, result) = match state.as_str() {
                "completed" if row.public => ("completed",None,Some(connection)),
                "failed"|"expired" => ("failed",Some(failure.ok_or(Failure::MetadataUnavailable)?),None),
                "pending"|"completing" if now>=expires => ("failed",Some("expired".to_owned()),None),
                "pending"|"completing" => ("pending",None,None),
                _ => return Err(Failure::MetadataUnavailable),
            };
            Ok(ObservedAcquisition {reference:reference.to_owned(),instance:instance.to_owned(),state,reason,connection:result,expires_at_ms:expires,observed_at_ms:now})
        })
    }

    /// Cursors are owner-held opaque records. A changed list epoch, selection,
    /// limit, configuration or original expiry refuses rather than shifting pages.
    pub fn list<'a>(
        &self,
        instance: &str,
        adapter: &str,
        configured: impl Into<Configured<'a>>,
        page: PageOptions<'_>,
        now: u64,
        custody_available: bool,
    ) -> Result<Page> {
        let configured = configured.into();
        let configuration_revision = configured.configuration_revision;
        let PageOptions { limit, cursor } = page;
        if !(1..=500).contains(&limit) {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, _, now| {
            tx.execute("DELETE FROM registry_cursors WHERE expires_at_ms<=?1", [timestamp(now)?]).map_err(db)?;
            let selection: Option<(String,i64)> = tx.query_row("SELECT adapter_id,epoch FROM registry_instances WHERE instance_id=?1", [instance], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(db)?;
            let epoch = match selection { Some((id,epoch)) if id==adapter => epoch, Some(_) => return Err(Failure::Conflict), None => 0 };
            let (last, expires) = if let Some(cursor) = cursor {
                if Uuid::parse_str(cursor).is_err() { return Err(Failure::StaleCursor); }
                tx.query_row("SELECT last_connection,expires_at_ms FROM registry_cursors WHERE cursor_id=?1 AND instance_id=?2 AND adapter_id=?3 AND configuration_revision=?4 AND page_limit=?5 AND epoch=?6",
                    params![cursor,instance,adapter,configuration_revision,limit,epoch], |r| Ok((r.get::<_,String>(0)?,read_time(r,1)?))).optional().map_err(db)?.ok_or(Failure::StaleCursor)?
            } else { (String::new(),deadline(now,ENTRY_MS)?) };
            let mut refs = tx.prepare("SELECT connection_ref FROM registry_connections WHERE instance_id=?1 AND public=1 AND connection_ref>?2 ORDER BY connection_ref LIMIT ?3").map_err(db)?
                .query_map(params![instance,last,u32::from(limit)+1], |r| r.get::<_,String>(0)).map_err(db)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db)?;
            let more = refs.len()>usize::from(limit);
            if more { refs.pop(); }
            let mut connections = Vec::with_capacity(refs.len());
            for reference in &refs {
                let row = Self::connection(tx, reference)?;
                visible(&row, instance, adapter)?;
                connections.push(observe(tx, row, &configured, now, custody_available)?);
            }
            let next_cursor = if more {
                let count: i64 = tx.query_row("SELECT count(*) FROM registry_cursors", [], |r| r.get(0)).map_err(db)?;
                if count>=10_000 { return Err(Failure::Capacity); }
                let next = new_id();
                tx.execute("INSERT INTO registry_cursors VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![next,instance,adapter,configuration_revision,epoch,refs.last().ok_or(Failure::MetadataUnavailable)?,limit,timestamp(expires)?]).map_err(db)?;
                Some(next)
            } else { None };
            let valid_until_ms = connections.iter().map(|c| c.valid_until_ms).min().unwrap_or(deadline(now,ENTRY_MS)?);
            let stale = connections.iter().any(|c| c.stale);
            Ok(Page {connections,next_cursor,observed_at_ms:now,valid_until_ms,stale})
        })
    }
}

pub(super) fn visible(row: &ConnectionRow, instance: &str, adapter: &str) -> Result<()> {
    if !row.public || row.binding.instance_id != instance || row.binding.adapter_id != adapter {
        return Err(Failure::NotFound);
    }
    Ok(())
}
fn observe(
    tx: &Transaction<'_>,
    row: ConnectionRow,
    configured: &Configured<'_>,
    now: u64,
    custody_available: bool,
) -> Result<ObservedConnection> {
    // A Stale connection (`lifecycle::mark_stale`) reports so before any
    // other state: the provider refused its credential, and only a reconnect's
    // publication changes that. It is not ready (`readiness`), so no read,
    // launch or write is admitted (contract connection section 4.1 row 3).
    let state = if row.state == "revoked" {
        State::Revoked
    } else if row.kept.stale {
        State::Stale
    } else if !configured.current(&row.binding) {
        State::Pending
    } else {
        readiness(tx, &row, now, custody_available)?
    };
    let stale = row.state != "revoked" && !configured.current(&row.binding);
    let valid_until_ms = if stale {
        now
    } else if state == State::Ready {
        row.baseline
            .as_ref()
            .ok_or(Failure::MetadataUnavailable)?
            .valid_until()
            .min(deadline(now, ENTRY_MS)?)
    } else {
        deadline(now, ENTRY_MS)?
    };
    Ok(ObservedConnection {
        reference: row.reference,
        instance: row.binding.instance_id,
        profile: row.binding.profile.id,
        revision: row.revision,
        state,
        identity: row.identity,
        observed_at_ms: now,
        valid_until_ms,
        stale,
    })
}

pub(super) fn readiness(
    tx: &Transaction<'_>,
    row: &ConnectionRow,
    now: u64,
    custody_available: bool,
) -> Result<State> {
    if row.state == "revoked" {
        return Ok(State::Revoked);
    }
    // Whatever made it Stale, the credential was refused: not ready, so every
    // caller that requires Ready (or admits Pending) refuses it as `NotReady`.
    if row.kept.stale {
        return Ok(State::Stale);
    }
    let Some(version) = &row.material else {
        return Ok(State::Pending);
    };
    let (ack,deleted,invalid,retired): (bool,bool,Option<String>,bool) = tx.query_row("SELECT acknowledged,deleted,invalid_reason,retirement_fence IS NOT NULL FROM registry_materials WHERE version_id=?1 AND connection_ref=?2 AND generation_id=?3",
        params![version,row.reference,row.generation], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(db)?;
    if !ack || retired {
        return Err(Failure::MetadataUnavailable);
    }
    let baseline = row.baseline.as_ref().ok_or(Failure::MetadataUnavailable)?;
    if deleted || invalid.is_some() || baseline.credential_expires_at.is_some_and(|e| now >= e) {
        return Ok(State::ReauthorizationRequired);
    }
    if !custody_available {
        return Ok(State::CustodyUnavailable);
    }
    if !baseline.current(now) {
        return Ok(State::Pending);
    }
    if !row.binding.profile.minimum_scopes.is_empty()
        && !baseline
            .granted_scopes
            .as_ref()
            .is_some_and(|s| row.binding.profile.minimum_scopes.is_subset(s))
    {
        return Ok(State::ReauthorizationRequired);
    }
    Ok(State::Ready)
}
