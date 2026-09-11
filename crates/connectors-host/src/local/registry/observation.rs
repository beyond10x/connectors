use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ready,
    Pending,
    ReauthorizationRequired,
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
    /// Resolve a retained approval target without sampling time, updating the
    /// registry clock, checking custody or granting a provider call. Expired
    /// validation evidence can identify this binding; known invalid material
    /// cannot. Invocation must independently establish current readiness.
    pub fn approval_target(
        &self,
        binding: &Binding,
        reference: &str,
        scopes: &BTreeSet<String>,
    ) -> Result<String> {
        if !connectors_core::valid_id(reference) {
            return Err(Failure::InvalidInput);
        }
        let mut metadata =
            Metadata::inspect(&self.path).map_err(|_| Failure::MetadataUnavailable)?;
        let tx = metadata.connection.transaction().map_err(db)?;
        let row = Self::connection(&tx, reference)?;
        visible(&row, &binding.instance_id, &binding.adapter_id)?;
        if row.state == "revoked" {
            return Err(Failure::Revoked);
        }
        if row.binding != *binding {
            return Err(Failure::Conflict);
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
        if deleted || invalid.is_some() || row.identity.is_none() {
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

    pub fn describe(
        &self,
        instance: &str,
        adapter: &str,
        configuration_revision: &str,
        reference: &str,
        now: u64,
        custody_available: bool,
    ) -> Result<ObservedConnection> {
        self.transaction(now, false, |tx, _, now| {
            let row = Self::connection(tx, reference)?;
            visible(&row, instance, adapter)?;
            observe(tx, row, configuration_revision, now, custody_available)
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
    pub fn list(
        &self,
        instance: &str,
        adapter: &str,
        configuration_revision: &str,
        page: PageOptions<'_>,
        now: u64,
        custody_available: bool,
    ) -> Result<Page> {
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
                connections.push(observe(tx, row, configuration_revision, now, custody_available)?);
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

fn visible(row: &ConnectionRow, instance: &str, adapter: &str) -> Result<()> {
    if !row.public || row.binding.instance_id != instance || row.binding.adapter_id != adapter {
        return Err(Failure::NotFound);
    }
    Ok(())
}
fn observe(
    tx: &Transaction<'_>,
    row: ConnectionRow,
    configured_revision: &str,
    now: u64,
    custody_available: bool,
) -> Result<ObservedConnection> {
    let state = if row.state == "revoked" {
        State::Revoked
    } else if row.binding.configuration_revision != configured_revision {
        State::Pending
    } else {
        readiness(tx, &row, now, custody_available)?
    };
    let stale = row.state != "revoked" && row.binding.configuration_revision != configured_revision;
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
