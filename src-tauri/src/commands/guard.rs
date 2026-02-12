//
// Signed-out guard session commands for revoke/recover flows.

use std::sync::Arc;

use secp256k1::SecretKey;
use tauri::State;
use tokio::sync::Mutex;

use crate::core::auth::GuardSessionManager;
use crate::core::channels::vrpc::identity as vrpc_identity;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::core::crypto::wif_encoding::decode_wif_unchecked_network;
use crate::core::crypto::{derive_keys_from_material, Network};
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use crate::types::{
    BeginGuardSessionRequest, BeginGuardSessionResult, EndGuardSessionRequest,
    EndGuardSessionResult, GuardIdentityPreflightRequest, GuardIdentitySendRequest,
    GuardPreflightResult, GuardSendResult, WalletError,
};

fn normalize_hex_private_key_candidate(input: &str) -> Option<String> {
    let stripped = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if stripped.len() != 64 || !stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let decoded = hex::decode(stripped).ok()?;
    if decoded.len() != 32 {
        return None;
    }
    SecretKey::from_slice(&decoded).ok()?;
    Some(stripped.to_lowercase())
}

fn classify_import_text(import_text: &str) -> Result<(WalletSecretKind, String), WalletError> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidImportText);
    }

    if decode_wif_unchecked_network(trimmed).is_ok() {
        return Ok((WalletSecretKind::Wif, trimmed.to_string()));
    }

    if let Some(private_key_hex) = normalize_hex_private_key_candidate(trimmed) {
        return Ok((WalletSecretKind::PrivateKeyHex, private_key_hex));
    }

    Ok((WalletSecretKind::SeedText, trimmed.to_string()))
}

fn to_derivation_network(network: WalletNetwork) -> Network {
    match network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    }
}

fn secret_kind_label(kind: WalletSecretKind) -> &'static str {
    match kind {
        WalletSecretKind::SeedText => "seed_text",
        WalletSecretKind::Wif => "wif",
        WalletSecretKind::PrivateKeyHex => "private_key_hex",
    }
}

/// Begin a temporary in-memory guard session from imported secret material.
#[tauri::command(rename_all = "snake_case")]
pub async fn begin_guard_session(
    request: BeginGuardSessionRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
) -> Result<BeginGuardSessionResult, WalletError> {
    let (secret_kind, normalized_secret) = classify_import_text(&request.import_text)?;
    let keys = derive_keys_from_material(
        &normalized_secret,
        secret_kind,
        to_derivation_network(request.network),
    )?;

    let mut guard = guard_session_manager.lock().await;
    let session = guard.begin_session(keys, request.network);

    Ok(BeginGuardSessionResult {
        guard_session_id: session.id,
        secret_kind: secret_kind_label(secret_kind).to_string(),
        vrsc_address: session.keys.address.clone(),
        eth_address: session.keys.eth_address.clone(),
        btc_address: session.keys.btc_address.clone(),
        network: session.network,
    })
}

/// End a guard session and zeroize its keys.
#[tauri::command(rename_all = "snake_case")]
pub async fn end_guard_session(
    request: EndGuardSessionRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
) -> Result<EndGuardSessionResult, WalletError> {
    let mut guard = guard_session_manager.lock().await;
    Ok(EndGuardSessionResult {
        ended: guard.end_session(&request.guard_session_id),
    })
}

/// Preflight identity update/revoke/recover in signed-out guard mode.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_guard_identity_update(
    request: GuardIdentityPreflightRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GuardPreflightResult, WalletError> {
    let guard_session = {
        let mut guard = guard_session_manager.lock().await;
        guard.get_session(&request.guard_session_id)?
    };

    let resolved = vrpc::parse_vrpc_channel_id(
        &request.params.channel_id,
        Some(&guard_session.keys.address),
    )?;
    if resolved.address != guard_session.keys.address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(guard_session.network, WalletNetwork::Testnet);
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let canonical_channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
    let guard_account_id = format!("guard:{}", request.guard_session_id);

    vrpc_identity::preflight(
        request.params,
        &preflight_store,
        &guard_account_id,
        &resolved.address,
        &canonical_channel_id,
        vrpc_provider_pool.for_network(guard_session.network),
    )
    .await
}

/// Broadcast identity tx in signed-out guard mode.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_guard_identity_update(
    request: GuardIdentitySendRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GuardSendResult, WalletError> {
    let guard_session = {
        let mut guard = guard_session_manager.lock().await;
        guard.get_session(&request.guard_session_id)?
    };

    let guard_account_id = format!("guard:{}", request.guard_session_id);
    vrpc_identity::send_with_signing_material(
        &request.preflight_id,
        &preflight_store,
        &guard_account_id,
        &guard_session.keys.wif,
        guard_session.network,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await
}
