//! Machine-wide custody inventory: every item this product wrote to the
//! Secret Service, by authority. It lists, removes the items of one authority
//! (a removed store's) and reports the items no store names. It reads item
//! attributes only, never a secret value, and never moves an item.
use super::*;
use std::collections::HashSet;
use uuid::Uuid;

/// One custody item as the Secret Service holds it. Attributes only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustodyItem {
    pub path: String,
    pub collection: String,
    pub purpose: Purpose,
    pub authority: String,
    pub allocation: String,
    pub version: String,
}

/// Items a removal deleted, and each it could not, named with its failure.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Removal {
    pub deleted: Vec<CustodyItem>,
    pub failed: Vec<(CustodyItem, Failure)>,
}

/// The items no store names; `removal` is present only when removal was asked.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Orphans {
    pub orphans: Vec<CustodyItem>,
    pub removal: Option<Removal>,
}

fn open(socket: Option<&Path>) -> Result<(Service, gnome::Persistence)> {
    guard_session_bus(socket);
    let service =
        Service::connect(local_stream_at(socket).map_err(unavailable)?).map_err(unavailable)?;
    if service.state().map_err(unavailable)? != State::Available {
        return Err(Failure::Unavailable);
    }
    let persistence = gnome::Persistence::admit(&service)?;
    persistence.check_process()?;
    persistence.check_encrypted()?;
    Ok((service, persistence))
}

fn list(service: &Service) -> Result<Vec<CustodyItem>> {
    let mut items = Vec::new();
    for collection in service.collections() {
        let proxy = service
            .proxy(collection.as_str(), COLLECTION_INTERFACE)
            .map_err(unavailable)?;
        for (schema, purpose) in [
            (CREDENTIAL_SCHEMA, Purpose::Credential),
            (SIGNING_SCHEMA, Purpose::ApprovalSigning),
        ] {
            let found: Vec<OwnedObjectPath> = proxy
                .call("SearchItems", &(HashMap::from([("xdg:schema", schema)]),))
                .map_err(unavailable)?;
            for path in found {
                let item = service
                    .proxy(path.as_str(), ITEM_INTERFACE)
                    .map_err(unavailable)?;
                let attributes: HashMap<String, String> =
                    item.get_property("Attributes").map_err(unavailable)?;
                let attribute = |name: &str| attributes.get(name).cloned().unwrap_or_default();
                items.push(CustodyItem {
                    path: path.to_string(),
                    collection: collection.to_string(),
                    purpose,
                    authority: attribute("connectors.authority"),
                    allocation: attribute("connectors.scope"),
                    version: attribute("connectors.version"),
                });
            }
        }
    }
    Ok(items)
}

fn remove(
    service: &Service,
    persistence: &gnome::Persistence,
    items: Vec<CustodyItem>,
) -> Result<Removal> {
    let _lock = persistence.lock(Instant::now() + Duration::from_secs(10))?;
    let mut removal = Removal::default();
    for item in items {
        let outcome = service
            .proxy(&item.path, ITEM_INTERFACE)
            .map_err(unavailable)
            .and_then(|proxy| {
                proxy
                    .call::<_, _, OwnedObjectPath>("Delete", &())
                    .map_err(unknown)
            })
            .and_then(|prompt| {
                if prompt.as_str() == "/" {
                    Ok(())
                } else {
                    Err(Failure::OutcomeUnknown)
                }
            });
        match outcome {
            Ok(()) => removal.deleted.push(item),
            Err(failure) => removal.failed.push((item, failure)),
        }
    }
    persistence
        .synchronize()
        .map_err(|_| Failure::OutcomeUnknown)?;
    Ok(removal)
}

/// Every custody item on the Secret Service, in the default collection and in
/// the legacy `login` collection when the alias resolves elsewhere.
pub fn items_at(socket: Option<&Path>) -> Result<Vec<CustodyItem>> {
    list(&open(socket)?.0)
}

/// Delete every custody item carrying `authority`: a removed store's
/// credential and approval signing items. A delete that fails is in
/// `failed`, named, never dropped.
pub fn remove_authority_at(socket: Option<&Path>, authority: Uuid) -> Result<Removal> {
    let (service, persistence) = open(socket)?;
    let wanted = authority.to_string();
    let items = list(&service)?
        .into_iter()
        .filter(|item| item.authority == wanted)
        .collect();
    remove(&service, &persistence, items)
}

/// The custody items whose authority is not in `known`. They are deleted only
/// when `remove` is true.
pub fn orphans_at(
    socket: Option<&Path>,
    known: &HashSet<Uuid>,
    remove_them: bool,
) -> Result<Orphans> {
    let (service, persistence) = open(socket)?;
    let orphans: Vec<CustodyItem> = list(&service)?
        .into_iter()
        .filter(|item| {
            item.authority
                .parse::<Uuid>()
                .map_or(true, |authority| !known.contains(&authority))
        })
        .collect();
    let removal = if remove_them {
        Some(remove(&service, &persistence, orphans.clone())?)
    } else {
        None
    };
    Ok(Orphans { orphans, removal })
}
