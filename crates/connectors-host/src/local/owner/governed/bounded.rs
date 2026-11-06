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
        // A projected read without a budget (the GovernedRead frame) holds the
        // effects rule too: the frame kind is the caller's to choose. Only that
        // rule; the budget's own checks stay with the budget. A projection that
        // cannot be read, or no longer lists the operation, is refused as the
        // bounded path refuses it, never dispatched with unknown effects.
        if let Some((policy, _)) = selection {
            let projected = policy.metadata(paths, alias, &plan.adapter, &plan.bootstrap)?;
            let metadata = projected
                .operations
                .get(request.operation)
                .ok_or(Code::NotFound)?;
            read_effects(&metadata.to_value().map_err(|_| Code::Unavailable)?)?;
        }
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
    read_effects(&value)?;
    let descriptor = plan.bootstrap.descriptor()?;
    let operation = descriptor
        .operation(request.operation)
        .map_err(|_| Code::NotFound)?;
    let profile = &operation.profile;
    // An implemented read runs only where the MCP projection would advertise
    // it: the `resource` profile over self-contained schemas. A caller that
    // sends a bounded-read frame directly reaches no wider set.
    let expected = match value["realization"].as_str() {
        Some("generic") if matches!(profile.as_str(), "generic-http" | "generic-http-page") => {
            (40_000, 30_000, 262_144, 4_194_304)
        }
        Some("implemented")
            if profile == "resource"
                && connectors_core::inline_schema(&operation.input_schema)
                && connectors_core::inline_schema(&operation.output_schema) =>
        {
            (20_000, 15_000, 65_536, 4_194_304)
        }
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

/// The owner holds the read rule itself rather than trusting a projection to
/// have hidden the operation: a declared semantic effect, or an effect other
/// than `network`, is never run as a projected read, with or without a budget.
fn read_effects(value: &Value) -> Result<()> {
    if value["semantic_effects"]
        .as_array()
        .is_none_or(|effects| !effects.is_empty())
        || value["effects"]
            .as_array()
            .is_none_or(|effects| effects.iter().any(|effect| effect != "network"))
    {
        return Err(Code::Unsupported.into());
    }
    Ok(())
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
