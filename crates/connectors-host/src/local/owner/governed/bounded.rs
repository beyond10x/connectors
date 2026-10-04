//! Current declaration selection for the explicit owner/native bounded read port.
use super::*;

pub(in crate::local::owner) fn resolve(
    paths: &Paths,
    alias: &str,
    request: &Request<'_>,
    selection: Selection<'_>,
    budget: Option<runtime::ReadBudget>,
) -> Result<ReadPlan> {
    let mut plan = resolve_selected(paths, alias, request, selection)?;
    let Some(budget) = budget else {
        return Ok(plan);
    };
    budget.until()?;
    let (policy, expected) = selection.ok_or(Code::Unsupported)?;
    if plan.adapter.private_protocol() != runtime::PrivateProtocol::V3 {
        return Err(Code::Unsupported.into());
    }
    let projected = policy.metadata(paths, alias, &plan.adapter, &plan.bootstrap)?;
    if projected.revision != expected {
        return Err(Code::StaleDescription.into());
    }
    let metadata = projected
        .operations
        .get(request.operation)
        .ok_or(Code::NotFound)?;
    let value = metadata.to_value().map_err(|_| Code::Unavailable)?;
    crate::local::operation_curation::bind(&plan.bootstrap, request.operation, &value)?;
    if value["approval"] != "not_required"
        || !matches!(
            value["idempotency"]["kind"].as_str(),
            Some("none" | "natural")
        )
    {
        return Err(Code::Unsupported.into());
    }
    let descriptor = plan.bootstrap.descriptor()?;
    let profile = &descriptor
        .operation(request.operation)
        .map_err(|_| Code::NotFound)?
        .profile;
    let expected = match value["realization"].as_str() {
        Some("generic") if matches!(profile.as_str(), "generic-http" | "generic-http-page") => {
            (40_000, 30_000, 262_144, 4_194_304)
        }
        Some("implemented") => (20_000, 15_000, 65_536, 4_194_304),
        _ => return Err(Code::Unsupported.into()),
    };
    let limits = &value["limits"];
    if budget.limits() != expected
        || limits["execution_ms"].as_u64() != Some(expected.0)
        || limits["provider_ms"].as_u64() != Some(expected.1)
        || limits["request_bytes"].as_u64() != Some(expected.2 as u64)
        || limits["result_bytes"].as_u64() != Some(expected.3 as u64)
        || limits["connect_ms"].as_u64() != Some(5000)
    {
        return Err(Code::Unsupported.into());
    }
    // The owner assigns a canonical UUID of this fixed ASCII length. This
    // measures the complete canonical service request, not just business input.
    let input = connectors_core::json::decode(request.input.as_bytes(), 64)
        .map_err(|_| Code::InvalidInput)?;
    let envelope = json!({"version":"v1alpha2","request_id":"00000000-0000-0000-0000-000000000001",
        "operation":request.operation,"revision":request.revision,"connection":request.connection,"input":input});
    if serde_json::to_vec(&envelope)
        .map_err(|_| Code::InvalidInput)?
        .len()
        > budget.input_bytes()
    {
        return Err(Code::InvalidInput.into());
    }
    plan.fingerprint = connectors_core::digest(
        &json!({"owner":plan.fingerprint,"metadata":value,"projection":projected.revision}),
    );
    Ok(plan)
}

pub(in crate::local::owner) fn read(
    paths: &Paths,
    host: &str,
    alias: &str,
    request: &Request<'_>,
    selection: (&dyn ReadPolicy, &str),
    budget: runtime::ReadBudget,
    dispatch: impl FnOnce(&Adapter) -> Result<Vec<u8>>,
) -> Result<Value> {
    let until = budget.until()?;
    let audits = audit::Store::new(&paths.state, 100_000).map_err(|_| Code::Unavailable)?;
    read_using(
        (paths, host, until),
        alias,
        request,
        &audits,
        Some(selection),
        Some(budget),
        dispatch,
    )
}
