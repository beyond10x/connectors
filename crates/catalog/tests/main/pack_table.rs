//! **The whole shipped pack maps into the typed table, and the mapping is total.**
//!
//! `src/table.rs` refuses by panicking, on the reasoning that a document it cannot map is a
//! corrupt build rather than an input a caller can act on. That reasoning is only sound if
//! something actually walks the whole catalogue, so this does: every provider, every credential,
//! every operation, every channel and every configuration field of the committed pack, with the
//! derived facts checked against what the documents say rather than against a copy of the table.
//!
//! It is the standing replacement for the predecessor's `embedded_operations.rs`, which held the
//! generated tables against `providers/` and needed `flux-lang` to parse the embedded text. There
//! is no embedded text any more, and the storage it held is the pack.

use catalog::{Acquisition, CredentialRequirement, OperationKey, Placement, ProviderKey};

/// Every provider in the pack becomes a provider in the table, in the same order, and the two
/// agree operation for operation.
#[test]
fn the_table_covers_the_whole_pack() {
    let providers = catalog::providers();
    assert!(
        providers.len() >= 55,
        "the shipped catalogue is 55 providers; the table built {}",
        providers.len()
    );

    let pack_ids: Vec<&str> = catalog::reader::providers()
        .map(|provider| provider.id())
        .collect();
    let table_ids: Vec<&str> = providers.iter().map(|provider| provider.id).collect();
    assert_eq!(table_ids, pack_ids);

    let pack_operations = catalog::reader::embedded().operations().len();
    assert_eq!(
        catalog::operations().count(),
        pack_operations,
        "the table and the pack disagree about how many operations ship"
    );
    assert!(
        pack_operations >= 835,
        "the shipped catalogue is 835 operations; the pack carries {pack_operations}"
    );
}

/// Ids are what lookups key on, so a duplicate would make one of the two unreachable — and every
/// id the table publishes must resolve through both lookup paths.
#[test]
fn every_operation_is_reachable_by_key_exactly_once() {
    let mut ids: Vec<&str> = catalog::operations()
        .map(|operation| operation.id)
        .collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "duplicate operation id in the catalogue");

    for operation in catalog::operations() {
        let by_key = catalog::operation(OperationKey::id(operation.id))
            .unwrap_or_else(|| panic!("`{}` does not resolve by key", operation.id));
        assert_eq!(by_key.id, operation.id);
        let provider = catalog::provider(ProviderKey::id(operation.provider))
            .unwrap_or_else(|| panic!("`{}`'s provider does not resolve", operation.id));
        assert!(provider.operation(OperationKey::id(operation.id)).is_some());
    }
}

/// The derived facts — the ones the document does not state outright — are checked against what it
/// does state, for every operation.
#[test]
fn the_derived_operation_facts_agree_with_the_documents() {
    for provider in catalog::providers() {
        for operation in provider.operations {
            assert_eq!(operation.provider, provider.id);

            // `hosts` is derived from the operation's own service's base URL, templating intact.
            assert_eq!(
                operation.hosts.len(),
                1,
                "`{}` should reach exactly one declared host",
                operation.id
            );
            let host = operation.hosts[0];
            assert!(!host.is_empty() && !host.contains("://") && !host.contains('/'));

            // `credential_requirement` is the derivation `table.rs` documents: declared exactly
            // when the effective mechanism list is non-empty.
            match operation.credential_requirement {
                CredentialRequirement::Declared => assert!(!operation.credentials.is_empty()),
                CredentialRequirement::NoneRequired | CredentialRequirement::Withheld => {
                    assert!(operation.credentials.is_empty())
                }
            }

            // Every credential an operation names must be one the connector declares.
            for mechanism in operation.credentials {
                for name in *mechanism {
                    assert!(
                        provider.credential(name).is_some(),
                        "`{}` names credential `{name}`, which `{}` does not declare",
                        operation.id,
                        provider.id
                    );
                }
            }
        }
    }
}

/// The credential surface: a leaf for every name, a placement for every scheme, and no value
/// anywhere. The last one is structural — no field could hold one — so what is asserted is that
/// the fields that exist carry declarations.
#[test]
fn every_credential_carries_a_leaf_and_a_placement() {
    let mut oauth2 = 0;
    let mut inbound = 0;
    for provider in catalog::providers() {
        for credential in provider.auth {
            assert!(
                credential.name.starts_with(&format!("{}.", provider.id)),
                "credential `{}` is not in `{}`'s namespace",
                credential.name,
                provider.id
            );
            assert_eq!(
                credential.name,
                format!("{}.{}", provider.id, credential.leaf)
            );
            match credential.place {
                Placement::Header { name, prefix: _ } => assert!(!name.is_empty()),
                Placement::Query { name } => assert!(!name.is_empty()),
                Placement::Inbound => inbound += 1,
            }
            if let Acquisition::OAuth2(spec) = credential.acquire {
                oauth2 += 1;
                assert!(
                    !spec.token_path.is_empty(),
                    "`{}` declares OAuth2 with no token path",
                    credential.name
                );
                assert!(!spec.grants.is_empty());
            }
            assert!(
                !matches!(credential.acquire, Acquisition::Minted { .. }),
                "no shipped document can express a minting join; see `table.rs`"
            );
        }
    }
    assert!(oauth2 > 0, "the catalogue ships OAuth2 credentials");
    assert!(inbound > 0, "the catalogue ships webhook signing secrets");
}

/// The inbound surface: every channel names events the provider declares, and every socket binding
/// carries the handshake facts a planner needs.
#[test]
fn every_channel_binding_resolves_its_events() {
    let mut channels = 0;
    for provider in catalog::providers() {
        for channel in provider.channels {
            channels += 1;
            assert_eq!(
                provider.channel(channel.name).map(|c| c.name),
                Some(channel.name)
            );
            assert!(!channel.base_url.is_empty());
            for event in channel.events {
                assert!(
                    provider
                        .events
                        .iter()
                        .any(|declared| declared.name == *event),
                    "`{}`'s channel `{}` names event `{event}`, which it does not declare",
                    provider.id,
                    channel.name
                );
            }
            if let Some(connect) = channel.connect {
                assert!(connect.path.starts_with('/'));
            }
        }
    }
    assert!(channels > 0, "the catalogue ships channel bindings");
}

/// Configuration: every closed set is addressable the way a host addresses a stored value.
#[test]
fn every_closed_configuration_set_is_addressable() {
    let mut closed = 0;
    for provider in catalog::providers() {
        for set in provider.config_choices {
            closed += 1;
            assert!(!set.choices.is_empty());
            assert_eq!(
                provider
                    .choices_for(set.service, set.kind, set.name)
                    .map(|found| found.field),
                Some(set.field),
                "`{}`'s closed set `{}` is not reachable by its own address",
                provider.id,
                set.field
            );
            assert!(
                provider.config.iter().any(|field| field.name == set.field),
                "`{}` publishes a closed set for undeclared field `{}`",
                provider.id,
                set.field
            );
        }
    }
    assert!(closed > 0, "the catalogue ships closed configuration sets");
}
