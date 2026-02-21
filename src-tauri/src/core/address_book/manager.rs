use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use bitcoin::{address::NetworkUnchecked, Network as BtcNetwork};
use ethers::types::Address;
use uuid::Uuid;
use zcash_client_backend::encoding::{decode_payment_address, encode_payment_address};
use zcash_protocol::constants::{mainnet, testnet};

use crate::types::address_book::{
    AddressBookContact, AddressBookEndpoint, AddressBookSnapshot, AddressEndpointKind,
    SaveAddressBookContactRequest,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const ADDRESS_BOOK_SCHEMA_VERSION: u8 = 1;
const MAX_CONTACTS: usize = 500;
const MAX_ENDPOINTS_PER_CONTACT: usize = 20;
const MAX_DISPLAY_NAME_LEN: usize = 64;
const MAX_NOTE_LEN: usize = 140;
const MAX_ENDPOINT_LABEL_LEN: usize = 32;

fn expected_btc_network(network: WalletNetwork) -> BtcNetwork {
    match network {
        WalletNetwork::Mainnet => BtcNetwork::Bitcoin,
        WalletNetwork::Testnet => BtcNetwork::Testnet,
    }
}

pub fn empty_snapshot() -> AddressBookSnapshot {
    AddressBookSnapshot {
        schema_version: ADDRESS_BOOK_SCHEMA_VERSION,
        contacts: vec![],
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn sorted_contacts(snapshot: &AddressBookSnapshot) -> Vec<AddressBookContact> {
    let mut contacts = snapshot.contacts.clone();
    contacts.sort_by(|a, b| {
        b.updated_at.cmp(&a.updated_at).then(
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase()),
        )
    });
    contacts
}

pub fn upsert_contact(
    snapshot: &mut AddressBookSnapshot,
    request: SaveAddressBookContactRequest,
    network: WalletNetwork,
) -> Result<AddressBookContact, WalletError> {
    if snapshot.contacts.len() >= MAX_CONTACTS && request.id.is_none() {
        return Err(WalletError::AddressBookInvalidInput);
    }

    let display_name = request.display_name.trim();
    if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_LEN {
        return Err(WalletError::AddressBookInvalidInput);
    }

    let mut note = request.note.map(|value| value.trim().to_string());
    if let Some(value) = note.as_ref() {
        if value.len() > MAX_NOTE_LEN {
            return Err(WalletError::AddressBookInvalidInput);
        }
    }
    if note.as_deref() == Some("") {
        note = None;
    }

    if request.endpoints.is_empty() || request.endpoints.len() > MAX_ENDPOINTS_PER_CONTACT {
        return Err(WalletError::AddressBookInvalidInput);
    }

    let existing_index = request.id.as_ref().and_then(|id| {
        snapshot
            .contacts
            .iter()
            .position(|contact| &contact.id == id)
    });

    if request.id.is_some() && existing_index.is_none() {
        return Err(WalletError::AddressBookContactNotFound);
    }

    let existing_contact = existing_index.map(|index| snapshot.contacts[index].clone());
    let existing_endpoint_map: HashMap<String, AddressBookEndpoint> = existing_contact
        .as_ref()
        .map(|contact| {
            contact
                .endpoints
                .iter()
                .map(|endpoint| (endpoint.id.clone(), endpoint.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut unique_keys = HashSet::<String>::new();
    for (index, contact) in snapshot.contacts.iter().enumerate() {
        if Some(index) == existing_index {
            continue;
        }

        for endpoint in &contact.endpoints {
            unique_keys.insert(endpoint_unique_key(
                endpoint.kind.clone(),
                &endpoint.normalized_address,
            ));
        }
    }

    let timestamp = now_unix();
    let mut endpoints = Vec::<AddressBookEndpoint>::with_capacity(request.endpoints.len());
    for input in request.endpoints {
        let label = input.label.trim();
        if label.is_empty() || label.len() > MAX_ENDPOINT_LABEL_LEN {
            return Err(WalletError::AddressBookInvalidInput);
        }

        let normalized_address =
            normalize_destination_address(input.kind.clone(), &input.address, network)?;
        let unique_key = endpoint_unique_key(input.kind.clone(), &normalized_address);
        if !unique_keys.insert(unique_key) {
            return Err(WalletError::AddressBookDuplicate);
        }

        let endpoint_id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let existing_endpoint = existing_endpoint_map.get(&endpoint_id);
        let created_at = existing_endpoint
            .map(|endpoint| endpoint.created_at)
            .unwrap_or(timestamp);
        let last_used_at = existing_endpoint.and_then(|endpoint| endpoint.last_used_at);

        endpoints.push(AddressBookEndpoint {
            id: endpoint_id,
            kind: input.kind,
            address: input.address.trim().to_string(),
            normalized_address,
            label: label.to_string(),
            last_used_at,
            created_at,
            updated_at: timestamp,
        });
    }

    let (contact_id, created_at) = if let Some(existing) = existing_contact.as_ref() {
        (existing.id.clone(), existing.created_at)
    } else {
        (
            request.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            timestamp,
        )
    };

    let contact = AddressBookContact {
        id: contact_id,
        display_name: display_name.to_string(),
        note,
        created_at,
        updated_at: timestamp,
        endpoints,
    };

    if let Some(index) = existing_index {
        snapshot.contacts[index] = contact.clone();
    } else {
        snapshot.contacts.push(contact.clone());
    }

    Ok(contact)
}

pub fn delete_contact(snapshot: &mut AddressBookSnapshot, contact_id: &str) -> bool {
    let before = snapshot.contacts.len();
    snapshot.contacts.retain(|contact| contact.id != contact_id);
    snapshot.contacts.len() != before
}

pub fn mark_endpoint_used(snapshot: &mut AddressBookSnapshot, endpoint_id: &str) -> bool {
    let timestamp = now_unix();
    for contact in &mut snapshot.contacts {
        for endpoint in &mut contact.endpoints {
            if endpoint.id == endpoint_id {
                endpoint.last_used_at = Some(timestamp);
                endpoint.updated_at = timestamp;
                contact.updated_at = timestamp;
                return true;
            }
        }
    }

    false
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn migrate_legacy_testnet_zs(value: &str, network: WalletNetwork) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if !normalized.starts_with(testnet::HRP_SAPLING_PAYMENT_ADDRESS) {
        return None;
    }

    let decoded =
        decode_payment_address(testnet::HRP_SAPLING_PAYMENT_ADDRESS, normalized.as_str()).ok()?;
    Some(encode_payment_address(
        sapling_payment_address_hrp(network),
        &decoded,
    ))
}

fn normalize_or_migrate_zs_endpoint(
    endpoint: &AddressBookEndpoint,
    network: WalletNetwork,
) -> Option<String> {
    normalize_destination_address(AddressEndpointKind::Zs, &endpoint.address, network)
        .ok()
        .or_else(|| migrate_legacy_testnet_zs(&endpoint.address, network))
        .or_else(|| {
            normalize_destination_address(
                AddressEndpointKind::Zs,
                &endpoint.normalized_address,
                network,
            )
            .ok()
        })
        .or_else(|| migrate_legacy_testnet_zs(&endpoint.normalized_address, network))
}

pub fn migrate_legacy_zs_snapshot(
    snapshot: &mut AddressBookSnapshot,
    network: WalletNetwork,
) -> bool {
    let mut changed = false;

    for contact in &mut snapshot.contacts {
        for endpoint in &mut contact.endpoints {
            if endpoint.kind != AddressEndpointKind::Zs {
                continue;
            }

            let Some(canonical) = normalize_or_migrate_zs_endpoint(endpoint, network) else {
                continue;
            };

            if endpoint.address != canonical || endpoint.normalized_address != canonical {
                endpoint.address = canonical.clone();
                endpoint.normalized_address = canonical;
                changed = true;
            }
        }
    }

    let mut best_zs_endpoint_id_by_key = HashMap::<String, (String, u64, u64)>::new();
    for contact in &snapshot.contacts {
        for endpoint in &contact.endpoints {
            if endpoint.kind != AddressEndpointKind::Zs {
                continue;
            }

            let key = endpoint_unique_key(endpoint.kind.clone(), &endpoint.normalized_address);
            let should_replace = match best_zs_endpoint_id_by_key.get(&key) {
                None => true,
                Some((best_id, best_updated_at, best_created_at)) => {
                    endpoint.updated_at > *best_updated_at
                        || (endpoint.updated_at == *best_updated_at
                            && endpoint.created_at > *best_created_at)
                        || (endpoint.updated_at == *best_updated_at
                            && endpoint.created_at == *best_created_at
                            && endpoint.id.as_str() < best_id.as_str())
                }
            };

            if should_replace {
                best_zs_endpoint_id_by_key.insert(
                    key,
                    (
                        endpoint.id.clone(),
                        endpoint.updated_at,
                        endpoint.created_at,
                    ),
                );
            }
        }
    }

    for contact in &mut snapshot.contacts {
        let before = contact.endpoints.len();
        contact.endpoints.retain(|endpoint| {
            if endpoint.kind != AddressEndpointKind::Zs {
                return true;
            }

            let key = endpoint_unique_key(endpoint.kind.clone(), &endpoint.normalized_address);
            best_zs_endpoint_id_by_key
                .get(&key)
                .map(|(best_id, _, _)| endpoint.id == *best_id)
                .unwrap_or(true)
        });
        if contact.endpoints.len() != before {
            changed = true;
        }
    }

    let contacts_before = snapshot.contacts.len();
    snapshot
        .contacts
        .retain(|contact| !contact.endpoints.is_empty());
    if snapshot.contacts.len() != contacts_before {
        changed = true;
    }

    changed
}

pub fn normalize_destination_address(
    kind: AddressEndpointKind,
    address: &str,
    network: WalletNetwork,
) -> Result<String, WalletError> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(WalletError::AddressBookInvalidInput);
    }

    match kind {
        AddressEndpointKind::Eth => {
            let parsed: Address = trimmed.parse().map_err(|_| WalletError::InvalidAddress)?;
            Ok(format!("{:#x}", parsed).to_ascii_lowercase())
        }
        AddressEndpointKind::Btc => {
            let parsed = trimmed
                .parse::<bitcoin::Address<NetworkUnchecked>>()
                .map_err(|_| WalletError::InvalidAddress)?;
            let checked = parsed
                .require_network(expected_btc_network(network))
                .map_err(|_| WalletError::InvalidAddress)?;
            Ok(checked.to_string())
        }
        AddressEndpointKind::Vrpc => {
            if let Some(name) = trimmed.strip_suffix('@') {
                if name.is_empty() {
                    return Err(WalletError::InvalidAddress);
                }
                let valid = name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
                if !valid {
                    return Err(WalletError::InvalidAddress);
                }
                return Ok(format!("{}@", name.to_ascii_lowercase()));
            }

            let length = trimmed.len();
            if !(25..=61).contains(&length) {
                return Err(WalletError::InvalidAddress);
            }

            let mut chars = trimmed.chars();
            let Some(prefix) = chars.next() else {
                return Err(WalletError::InvalidAddress);
            };
            if prefix != 'R' && prefix != 'i' {
                return Err(WalletError::InvalidAddress);
            }

            if !chars.all(is_base58_char) {
                return Err(WalletError::InvalidAddress);
            }

            Ok(trimmed.to_string())
        }
        AddressEndpointKind::Zs => {
            let hrp = sapling_payment_address_hrp(network);
            let normalized = trimmed.to_ascii_lowercase();
            let decoded = decode_payment_address(hrp, normalized.as_str())
                .map_err(|_| WalletError::InvalidAddress)?;
            Ok(encode_payment_address(hrp, &decoded))
        }
    }
}

fn endpoint_unique_key(kind: AddressEndpointKind, normalized_address: &str) -> String {
    format!("{:?}:{}", kind, normalized_address)
}

fn is_base58_char(ch: char) -> bool {
    matches!(
        ch,
        '1'..='9'
            | 'A'..='H'
            | 'J'..='N'
            | 'P'..='Z'
            | 'a'..='k'
            | 'm'..='z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_eth_address_to_lowercase() {
        let normalized = normalize_destination_address(
            AddressEndpointKind::Eth,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            WalletNetwork::Mainnet,
        )
        .expect("valid eth");
        assert_eq!(normalized, "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    }

    #[test]
    fn rejects_duplicate_endpoints_across_contacts() {
        let mut snapshot = empty_snapshot();
        let first = SaveAddressBookContactRequest {
            id: None,
            display_name: "Alice".to_string(),
            note: None,
            endpoints: vec![crate::types::address_book::SaveAddressBookEndpointInput {
                id: None,
                kind: AddressEndpointKind::Vrpc,
                address: "RDA4cpQ5Qf8N6gJY8m5D7QJxX7P7Y9FNh2".to_string(),
                label: "Main".to_string(),
            }],
        };
        let _ = upsert_contact(&mut snapshot, first, WalletNetwork::Mainnet).expect("saved");

        let second = SaveAddressBookContactRequest {
            id: None,
            display_name: "Bob".to_string(),
            note: None,
            endpoints: vec![crate::types::address_book::SaveAddressBookEndpointInput {
                id: None,
                kind: AddressEndpointKind::Vrpc,
                address: "RDA4cpQ5Qf8N6gJY8m5D7QJxX7P7Y9FNh2".to_string(),
                label: "Main".to_string(),
            }],
        };

        let result = upsert_contact(&mut snapshot, second, WalletNetwork::Mainnet);
        assert!(matches!(result, Err(WalletError::AddressBookDuplicate)));
    }

    #[test]
    fn normalize_btc_bech32_accepts_mainnet_and_lowercases() {
        let normalized = normalize_destination_address(
            AddressEndpointKind::Btc,
            "BC1QGGQZJ0UZUN238NHZZS5WDZ2EN05S0D9NCWHXCF",
            WalletNetwork::Mainnet,
        )
        .expect("valid mainnet bech32");

        assert_eq!(normalized, "bc1qggqzj0uzun238nhzzs5wdz2en05s0d9ncwhxcf");
    }

    #[test]
    fn normalize_btc_bech32_rejects_network_mismatch() {
        let result = normalize_destination_address(
            AddressEndpointKind::Btc,
            "bc1qggqzj0uzun238nhzzs5wdz2en05s0d9ncwhxcf",
            WalletNetwork::Testnet,
        );
        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn normalize_btc_bech32_accepts_testnet() {
        let normalized = normalize_destination_address(
            AddressEndpointKind::Btc,
            "tb1qggqzj0uzun238nhzzs5wdz2en05s0d9njgv4r6",
            WalletNetwork::Testnet,
        )
        .expect("valid testnet bech32");

        assert_eq!(normalized, "tb1qggqzj0uzun238nhzzs5wdz2en05s0d9njgv4r6");
    }

    #[test]
    fn normalize_zs_mainnet_accepts_valid_address() {
        let normalized = normalize_destination_address(
            AddressEndpointKind::Zs,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z",
            WalletNetwork::Mainnet,
        )
        .expect("valid mainnet sapling address");

        assert_eq!(
            normalized,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z"
        );
    }

    #[test]
    fn normalize_zs_testnet_accepts_valid_address() {
        let normalized = normalize_destination_address(
            AddressEndpointKind::Zs,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z",
            WalletNetwork::Testnet,
        )
        .expect("valid testnet sapling address");

        assert_eq!(
            normalized,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z"
        );
    }

    #[test]
    fn normalize_zs_testnet_rejects_legacy_prefix() {
        let result = normalize_destination_address(
            AddressEndpointKind::Zs,
            "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk",
            WalletNetwork::Testnet,
        );
        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn migrate_legacy_zs_snapshot_converts_and_deduplicates() {
        let mut snapshot = AddressBookSnapshot {
            schema_version: 1,
            contacts: vec![
                AddressBookContact {
                    id: "legacy-contact".to_string(),
                    display_name: "Legacy".to_string(),
                    note: None,
                    created_at: 10,
                    updated_at: 10,
                    endpoints: vec![AddressBookEndpoint {
                        id: "legacy-endpoint".to_string(),
                        kind: AddressEndpointKind::Zs,
                        address: "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk".to_string(),
                        normalized_address: "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk".to_string(),
                        label: "Legacy".to_string(),
                        last_used_at: None,
                        created_at: 10,
                        updated_at: 10,
                    }],
                },
                AddressBookContact {
                    id: "current-contact".to_string(),
                    display_name: "Current".to_string(),
                    note: None,
                    created_at: 20,
                    updated_at: 20,
                    endpoints: vec![AddressBookEndpoint {
                        id: "current-endpoint".to_string(),
                        kind: AddressEndpointKind::Zs,
                        address: "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z".to_string(),
                        normalized_address: "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z".to_string(),
                        label: "Current".to_string(),
                        last_used_at: None,
                        created_at: 20,
                        updated_at: 20,
                    }],
                },
            ],
        };

        let changed = migrate_legacy_zs_snapshot(&mut snapshot, WalletNetwork::Testnet);
        assert!(changed);
        assert_eq!(snapshot.contacts.len(), 1);
        assert_eq!(snapshot.contacts[0].id, "current-contact");
        assert_eq!(snapshot.contacts[0].endpoints.len(), 1);
        assert_eq!(snapshot.contacts[0].endpoints[0].id, "current-endpoint");
        assert_eq!(
            snapshot.contacts[0].endpoints[0].address,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z"
        );
    }

    #[test]
    fn migrate_legacy_zs_snapshot_converts_when_unique() {
        let mut snapshot = AddressBookSnapshot {
            schema_version: 1,
            contacts: vec![AddressBookContact {
                id: "legacy-contact".to_string(),
                display_name: "Legacy".to_string(),
                note: None,
                created_at: 10,
                updated_at: 10,
                endpoints: vec![AddressBookEndpoint {
                    id: "legacy-endpoint".to_string(),
                    kind: AddressEndpointKind::Zs,
                    address: "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk".to_string(),
                    normalized_address: "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk".to_string(),
                    label: "Legacy".to_string(),
                    last_used_at: None,
                    created_at: 10,
                    updated_at: 10,
                }],
            }],
        };

        let changed = migrate_legacy_zs_snapshot(&mut snapshot, WalletNetwork::Testnet);
        assert!(changed);
        assert_eq!(snapshot.contacts.len(), 1);
        assert_eq!(snapshot.contacts[0].endpoints.len(), 1);
        assert_eq!(
            snapshot.contacts[0].endpoints[0].address,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z"
        );
        assert_eq!(
            snapshot.contacts[0].endpoints[0].normalized_address,
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z"
        );
    }
}
