use super::*;

/// Checks every flow graph: that its references resolve, and that it has a lowering at all.
///
/// **The structural rules are not style.** Flux has no `goto`, so a cyclic graph and a graph whose
/// control regions overlap have no expressible form — a compiler that accepted them would have to
/// guess, and guessing produces plausible-but-wrong Flux, which is the one output this pipeline
/// refuses everywhere else.
pub(super) fn validate_graphs(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for graph in &connector.graphs {
        let name = graph.name.as_str();
        if name.trim().is_empty() {
            problems.push("a graph has an empty `name`".to_owned());
            continue;
        }
        if seen.contains(&name) {
            problems.push(format!(
                "graph {name:?} is declared more than once; the name becomes an emitted `op`, so it \
                 must denote one flow"
            ));
        }
        seen.push(name);

        if let Err(reason) = crate::address::validate_member_name(name) {
            problems.push(format!("graph {name:?} has an invalid `name`: {reason}"));
        }
        validate_member_service(connector, "graph", name, &graph.service, problems);

        validate_graph_nodes(connector, graph, problems);
        validate_graph_structure(graph, problems);
        validate_graph_edges(graph, problems);
    }
}

/// Every node's references resolve, in the graph's own service.
fn validate_graph_nodes(connector: &Connector, graph: &Graph, problems: &mut Vec<String>) {
    let name = graph.name.as_str();
    let mut ids: Vec<&str> = Vec::new();

    for node in &graph.nodes {
        let id = node.id.as_str();
        if id.trim().is_empty() {
            problems.push(format!("graph {name:?} has a node with an empty `id`"));
            continue;
        }
        if ids.contains(&id) {
            problems.push(format!(
                "graph {name:?} declares node {id:?} more than once; a node id is the stable \
                 identity an editor and a diagnostic both key on"
            ));
        }
        ids.push(id);

        match &node.kind {
            NodeKind::Operation { operation } => {
                resolve_member(
                    connector,
                    graph,
                    name,
                    id,
                    "operation",
                    operation,
                    connector
                        .operation(operation)
                        .map(|target| target.service.as_str()),
                    problems,
                );
            }
            NodeKind::Trigger { event } => {
                resolve_member(
                    connector,
                    graph,
                    name,
                    id,
                    "event",
                    event,
                    connector.event(event).map(|target| target.service.as_str()),
                    problems,
                );
            }
            NodeKind::Endpoint { binding } => {
                resolve_member(
                    connector,
                    graph,
                    name,
                    id,
                    "channel binding",
                    binding,
                    connector
                        .channel(binding)
                        .map(|target| target.service.as_str()),
                    problems,
                );
            }
            NodeKind::Select { path } => {
                if let Err(reason) = crate::inbound::validate_path(path) {
                    problems.push(format!(
                        "graph {name:?} node {id:?} selects an invalid path: {reason}"
                    ));
                }
            }
            NodeKind::Object { fields } => {
                for (field, port) in fields {
                    if !node.inputs.iter().any(|p| &p.name == port) {
                        problems.push(format!(
                            "graph {name:?} node {id:?} builds field {field:?} from port {port:?}, \
                             which it does not declare as an input"
                        ));
                    }
                }
            }
            NodeKind::Retry { max, .. } if *max == 0 => problems.push(format!(
                "graph {name:?} node {id:?} retries 0 times. flux's analyzer rejects unbounded loops \
                 and a zero bound is not a loop at all — remove the node or give it a real maximum"
            )),
            NodeKind::Throttle { max, window_ms } if *max == 0 || *window_ms == 0 => {
                problems.push(format!(
                    "graph {name:?} node {id:?} throttles to {max} per {window_ms}ms, which admits \
                     nothing. A throttle bounds a rate; it is not a way to disable a branch"
                ));
            }
            _ => {}
        }

        // A boundary node declares what wakes the flow. It is emitted nowhere, so it can neither
        // consume a value nor sit inside a region that only exists at runtime.
        if node.kind.is_boundary() {
            if !node.inputs.is_empty() {
                problems.push(format!(
                    "graph {name:?} node {id:?} is a `{}` boundary and declares inputs. A boundary \
                     says what wakes the flow; nothing inside the flow can feed it",
                    node.kind.word()
                ));
            }
            if node.region.is_some() {
                problems.push(format!(
                    "graph {name:?} node {id:?} is a `{}` boundary inside a region. A boundary is \
                     emitted nowhere, so it cannot be conditional, retried or rate-limited",
                    node.kind.word()
                ));
            }
        }

        // The rule with teeth. See the module docs on `graph`.
        if matches!(node.kind, NodeKind::Gate { .. }) && !node.outputs.is_empty() {
            problems.push(format!(
                "graph {name:?} node {id:?} is a gate declaring outputs. A gate lowers to Flux's \
                 `when`, which has no else branch here — a symbol bound inside it is *unbound* when \
                 the condition is false, and reading it afterwards fails at runtime. A value that \
                 must escape a conditional needs a branch with a default"
            ));
        }
        if !node.kind.is_region() && !node.outputs.is_empty() && node.region.is_some() {
            // Non-region nodes may have outputs; this only checks that they are reachable, which
            // `validate_graph_edges` covers. Nothing to add here.
        }
    }
}

/// One member reference: it exists, and it belongs to this graph's service.
#[allow(clippy::too_many_arguments)]
fn resolve_member(
    _connector: &Connector,
    graph: &Graph,
    name: &str,
    id: &str,
    kind: &str,
    reference: &str,
    found: Option<&str>,
    problems: &mut Vec<String>,
) {
    match found {
        None => problems.push(format!(
            "graph {name:?} node {id:?} names {kind} {reference:?}, which this connector does not \
             declare"
        )),
        Some(service) if service != graph.service => problems.push(format!(
            "graph {name:?} is in service {:?} but node {id:?} names {kind} {reference:?}, which is \
             in service {service:?}",
            graph.service
        )),
        Some(_) => {}
    }
}

/// No cycles, and every region containment resolves.
fn validate_graph_structure(graph: &Graph, problems: &mut Vec<String>) {
    let name = graph.name.as_str();

    for node in &graph.nodes {
        let Some(region) = node.region.as_deref() else {
            continue;
        };
        match graph.node(region) {
            None => problems.push(format!(
                "graph {name:?} node {:?} names region {region:?}, which is not a node of this graph",
                node.id
            )),
            Some(container) if !container.kind.is_region() => problems.push(format!(
                "graph {name:?} node {:?} is inside {region:?}, which is a `{}` and contains nothing",
                node.id,
                container.kind.word()
            )),
            Some(_) => {}
        }
        if graph.enclosing(&node.id).is_none() {
            problems.push(format!(
                "graph {name:?} node {:?} is contained in itself, directly or through a chain of \
                 regions",
                node.id
            ));
        }
    }

    if graph.topological_order().is_none() {
        problems.push(format!(
            "graph {name:?} has a cycle. Flux has no `goto` and its control flow is strictly nested, \
             so a cyclic graph has no lowering at all — an iteration is a bounded loop node, not an \
             edge pointing backwards"
        ));
    }
}

/// Every edge connects declared ports, and no edge crosses a region boundary.
fn validate_graph_edges(graph: &Graph, problems: &mut Vec<String>) {
    let name = graph.name.as_str();

    for edge in &graph.edges {
        let from = endpoint(graph, name, &edge.from, Side::Output, problems);
        let to = endpoint(graph, name, &edge.to, Side::Input, problems);
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };

        // A value may enter a region freely — an inner statement reads an outer symbol, which Flux
        // allows. It may only *leave* through a port the region declares, because that is the one
        // place a bound symbol is guaranteed to exist after the block closes.
        let (Some(source_regions), Some(sink_regions)) =
            (graph.enclosing(&from.id), graph.enclosing(&to.id))
        else {
            continue; // a containment cycle, already reported
        };

        for region in &source_regions {
            if sink_regions.contains(region) {
                continue; // the sink is inside the same region; nothing escapes
            }
            let Some(container) = graph.node(region) else {
                continue;
            };
            let escapes_through = container
                .outputs
                .iter()
                .any(|port| port.name == edge.from.port);
            if !escapes_through {
                problems.push(format!(
                    "graph {name:?} has an edge from {:?}.{:?} out of region {region:?} to {:?}, but \
                     {region:?} declares no output port {:?}. A value leaves a region only through a \
                     port the region declares — otherwise the symbol it lowers to may not be bound \
                     when the block closes",
                    from.id, edge.from.port, to.id, edge.from.port
                ));
            }
        }
    }

    if let Some(output) = &graph.output {
        endpoint(graph, name, output, Side::Output, problems);
    }
}

enum Side {
    Input,
    Output,
}

/// Resolve one end of an edge, reporting a missing node or a missing port.
fn endpoint<'a>(
    graph: &'a Graph,
    name: &str,
    reference: &PortRef,
    side: Side,
    problems: &mut Vec<String>,
) -> Option<&'a GraphNode> {
    let Some(node) = graph.node(&reference.node) else {
        problems.push(format!(
            "graph {name:?} has an edge naming node {:?}, which it does not declare",
            reference.node
        ));
        return None;
    };
    let (ports, word) = match side {
        Side::Input => (&node.inputs, "input"),
        Side::Output => (&node.outputs, "output"),
    };
    if !ports.iter().any(|port| port.name == reference.port) {
        problems.push(format!(
            "graph {name:?} node {:?} has no {word} port {:?}",
            reference.node, reference.port
        ));
    }
    Some(node)
}
