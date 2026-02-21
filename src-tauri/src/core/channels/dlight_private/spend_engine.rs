use rand::rngs::OsRng;
use std::convert::Infallible;
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};
use zcash_client_backend::proto::service::{
    self, compact_tx_streamer_client::CompactTxStreamerClient,
};
use zcash_primitives::transaction::builder::{BuildConfig, Builder, Error as TxBuildError};
use zcash_primitives::transaction::fees::fixed;
use zcash_protocol::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters};
use zcash_protocol::memo::MemoBytes;
use zcash_protocol::value::Zatoshis;
use zip32::Scope;

use crate::core::channels::dlight_private::destination::DlightDestinationKind;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

use super::recipient_resolution::{resolve_dlight_recipient, ResolvedDlightRecipient};
use super::spend_keys::DlightSpendKeyMaterial;
use super::spend_params::load_sapling_provers;
use super::spend_sync::{load_spend_snapshot, mark_notes_spent, SpendableNote};
use super::{normalize_grpc_endpoint, DlightRuntimeRequest};
use crate::core::channels::vrpc::VrpcProvider;

pub const SATOSHIS_PER_COIN: i128 = 100_000_000;
pub const FIXED_FEE_SATS: i128 = 10_000;

const DIAL_CONNECT_TIMEOUT_SECS: u64 = 10;
const DIAL_RPC_TIMEOUT_SECS: u64 = 20;
const VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT: u64 = 227_520;
const VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT: u64 = 1;

#[derive(Debug, Clone, Copy)]
struct VerusConsensusParams {
    network_type: NetworkType,
    sapling_activation_height: BlockHeight,
}

impl Parameters for VerusConsensusParams {
    fn network_type(&self) -> NetworkType {
        self.network_type
    }

    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match nu {
            NetworkUpgrade::Overwinter => Some(self.sapling_activation_height),
            NetworkUpgrade::Sapling => Some(self.sapling_activation_height),
            NetworkUpgrade::Blossom => None,
            NetworkUpgrade::Heartwood => None,
            NetworkUpgrade::Canopy => None,
            NetworkUpgrade::Nu5 => None,
            NetworkUpgrade::Nu6 => None,
            NetworkUpgrade::Nu6_1 => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DlightPreflightComputation {
    pub resolved_recipient: ResolvedDlightRecipient,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub value: String,
    pub fee: String,
    pub fee_taken_from_amount: bool,
    pub fee_taken_message: Option<String>,
    pub memo: Option<String>,
}

pub async fn compute_preflight(
    request: &DlightRuntimeRequest,
    to_address: &str,
    amount: &str,
    memo: Option<String>,
    confirmed_balance_sats: u64,
    vrpc_provider: &VrpcProvider,
) -> Result<DlightPreflightComputation, WalletError> {
    DlightSpendKeyMaterial::from_seed_material(
        &request.seed_material,
        request.network,
        &request.scope_address,
    )?;
    let resolved_recipient =
        resolve_dlight_recipient(to_address, request.network, vrpc_provider).await?;

    let submitted_sat = parse_positive_satoshis(amount)?;
    let confirmed_balance = i128::from(confirmed_balance_sats);
    if confirmed_balance <= 0 {
        return Err(WalletError::InsufficientFunds);
    }

    let (value_sat_i128, fee_taken_from_amount, fee_taken_message) =
        resolve_send_value(submitted_sat, confirmed_balance, FIXED_FEE_SATS)?;
    let value_sats = u64::try_from(value_sat_i128).map_err(|_| WalletError::OperationFailed)?;
    let fee_sats = u64::try_from(FIXED_FEE_SATS).map_err(|_| WalletError::OperationFailed)?;

    let normalized_memo = if resolved_recipient.is_shielded() {
        normalize_optional_memo(memo)
    } else {
        None
    };

    Ok(DlightPreflightComputation {
        resolved_recipient,
        value_sats,
        fee_sats,
        value: satoshis_to_decimal_string(value_sat_i128),
        fee: satoshis_to_decimal_string(FIXED_FEE_SATS),
        fee_taken_from_amount,
        fee_taken_message,
        memo: normalized_memo,
    })
}

#[derive(Debug, Clone)]
pub struct ExecuteSendParams {
    pub destination_kind: DlightDestinationKind,
    pub display_to_address: String,
    pub delivery_to_address: String,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlightSendStage {
    SyncingSpendState,
    LoadingProver,
    BuildingProof,
    Broadcasting,
}

#[derive(Debug, Clone)]
pub struct ExecutedSend {
    pub txid: String,
}

pub async fn execute_send(
    request: &DlightRuntimeRequest,
    params: &ExecuteSendParams,
    progress: Option<&(dyn Fn(DlightSendStage) + Send + Sync)>,
) -> Result<ExecutedSend, WalletError> {
    let key_material = DlightSpendKeyMaterial::from_seed_material(
        &request.seed_material,
        request.network,
        &request.scope_address,
    )?;
    let runtime_snapshot =
        super::runtime::get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    let runtime_tip_hint = runtime_snapshot
        .chain_tip_height
        .or(runtime_snapshot.estimated_tip_height)
        .filter(|tip| *tip > 0);
    let sync_snapshot = load_spend_snapshot(request, runtime_tip_hint)?;

    let total_required = params
        .value_sats
        .checked_add(params.fee_sats)
        .ok_or_else(|| {
            dlight_send_failure("amount calculation", "value plus fee overflowed u64 limits")
        })?;

    let (selected_notes, selected_total) =
        select_notes(&sync_snapshot.spendable_notes, total_required)
            .ok_or(WalletError::InsufficientFunds)?;
    let change_sats = selected_total.saturating_sub(total_required);

    match request.network {
        WalletNetwork::Mainnet => {
            execute_send_for_network(
                request,
                params,
                &key_material,
                &selected_notes,
                change_sats,
                sync_snapshot.chain_tip_height,
                verus_send_params(WalletNetwork::Mainnet),
                progress,
            )
            .await
        }
        WalletNetwork::Testnet => {
            execute_send_for_network(
                request,
                params,
                &key_material,
                &selected_notes,
                change_sats,
                sync_snapshot.chain_tip_height,
                verus_send_params(WalletNetwork::Testnet),
                progress,
            )
            .await
        }
    }
}

async fn execute_send_for_network<P: Parameters + Send + Clone>(
    request: &DlightRuntimeRequest,
    params: &ExecuteSendParams,
    key_material: &DlightSpendKeyMaterial,
    selected_notes: &[SpendableNote],
    change_sats: u64,
    chain_tip_height: u64,
    network: P,
    progress: Option<&(dyn Fn(DlightSendStage) + Send + Sync)>,
) -> Result<ExecutedSend, WalletError> {
    let anchor = selected_notes
        .first()
        .map(|note| {
            let node = sapling::Node::from_cmu(&note.note.cmu());
            sapling::Anchor::from(note.merkle_path.root(node))
        })
        .ok_or(WalletError::InsufficientFunds)?;

    let target_height = chain_tip_height.saturating_add(1);
    let target_height = BlockHeight::from_u32(
        u32::try_from(target_height).map_err(|_| {
            dlight_send_failure(
                "target height selection",
                "next block height could not be represented as u32",
            )
        })?,
    );

    let build_config = BuildConfig::Standard {
        sapling_anchor: Some(anchor),
        orchard_anchor: None,
    };
    let mut builder = Builder::new(network, target_height, build_config);

    for note in selected_notes {
        let fvk = key_material.sapling_fvk_for_scope(note.scope);
        builder
            .add_sapling_spend::<Infallible>(fvk, note.note.clone(), note.merkle_path.clone())
            .map_err(map_build_error)?;
    }

    let send_value =
        Zatoshis::from_u64(params.value_sats).map_err(|_| {
            dlight_send_failure("amount encoding", "send amount could not be represented as zatoshis")
        })?;
    match params.destination_kind {
        DlightDestinationKind::Shielded => {
            let recipient = decode_shielded_delivery(&params.delivery_to_address, request.network)?;
            let memo_bytes = parse_memo_bytes(params.memo.as_deref())?;
            builder
                .add_sapling_output::<Infallible>(
                    Some(key_material.sapling_ovk_for_scope(Scope::External)),
                    recipient,
                    send_value,
                    memo_bytes,
                )
                .map_err(map_build_error)?;
        }
        DlightDestinationKind::Transparent => {
            let recipient = decode_transparent_delivery(&params.delivery_to_address)?;
            builder
                .add_transparent_output(&recipient, send_value)
                .map_err(|error| {
                    dlight_send_failure(
                        "transparent output creation",
                        format!("transparent output builder rejected destination: {error}"),
                    )
                })?;
        }
    }

    if change_sats > 0 {
        let change_value = Zatoshis::from_u64(change_sats).map_err(|_| {
            dlight_send_failure(
                "change encoding",
                "change amount could not be represented as zatoshis",
            )
        })?;
        builder
            .add_sapling_output::<Infallible>(
                Some(key_material.sapling_ovk_for_scope(Scope::Internal)),
                key_material.change_payment_address(),
                change_value,
                MemoBytes::empty(),
            )
            .map_err(map_build_error)?;
    }

    // A spend set may include both external and internal (change) notes.
    // Provide both keys so Sapling builder can authorize either scope.
    let sapling_extsks = key_material.sapling_extsks_for_builder();
    let fee_rule = fixed::FeeRule::non_standard(
        Zatoshis::from_u64(params.fee_sats).map_err(|_| {
            dlight_send_failure("fee encoding", "fee could not be represented as zatoshis")
        })?,
    );
    emit_send_stage(progress, DlightSendStage::LoadingProver);
    let proving = load_sapling_provers()?;

    emit_send_stage(progress, DlightSendStage::BuildingProof);
    let build_result = builder
        .build(
            &zcash_transparent::builder::TransparentSigningSet::new(),
            &sapling_extsks,
            &[],
            OsRng,
            &proving.spend,
            &proving.output,
            &fee_rule,
        )
        .map_err(map_build_error)?;

    let tx = build_result.transaction();
    let txid = tx.txid().to_string();

    let mut raw = Vec::<u8>::new();
    tx.write(&mut raw)
        .map_err(|error| {
            dlight_send_failure(
                "transaction serialization",
                format!("failed to serialize transaction bytes: {error}"),
            )
        })?;
    eprintln!(
        "[dlight_private][spend_send] built tx txid={} bytes={}",
        txid,
        raw.len()
    );
    emit_send_stage(progress, DlightSendStage::Broadcasting);
    broadcast_transaction(&request.endpoint, raw).await?;

    let spent_height = chain_tip_height.saturating_add(1);
    let spent_nullifiers = selected_notes
        .iter()
        .map(|note| note.nullifier_hex.clone())
        .collect::<Vec<_>>();
    if let Err(err) = mark_notes_spent(request, &spent_nullifiers, spent_height) {
        eprintln!(
            "[dlight_private][spend_send] failed to mark notes spent in cache: {:?}",
            err
        );
    }

    Ok(ExecutedSend { txid })
}

fn emit_send_stage(
    progress: Option<&(dyn Fn(DlightSendStage) + Send + Sync)>,
    stage: DlightSendStage,
) {
    if let Some(report) = progress {
        report(stage);
    }
}

pub fn normalize_optional_memo(memo: Option<String>) -> Option<String> {
    memo.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn resolve_send_value(
    submitted_sat: i128,
    confirmed_balance: i128,
    fee_sat: i128,
) -> Result<(i128, bool, Option<String>), WalletError> {
    if submitted_sat <= 0 || confirmed_balance <= 0 || fee_sat <= 0 {
        return Err(WalletError::OperationFailed);
    }

    let deducted_amount = submitted_sat.saturating_add(fee_sat);

    if deducted_amount == confirmed_balance.saturating_add(fee_sat) {
        let adjusted = submitted_sat.saturating_sub(fee_sat);
        if adjusted <= 0 {
            return Err(WalletError::InsufficientFunds);
        }
        return Ok((
            adjusted,
            true,
            Some(
                "Fee was deducted from the submitted amount due to available balance.".to_string(),
            ),
        ));
    }

    if deducted_amount > confirmed_balance {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((submitted_sat, false, None))
}

pub fn parse_positive_satoshis(value: &str) -> Result<i128, WalletError> {
    let parsed = parse_decimal_to_satoshis(value)?;
    if parsed <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(parsed)
}

pub fn parse_decimal_to_satoshis(value: &str) -> Result<i128, WalletError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let (is_negative, numeric) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };
    if numeric.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let mut parts = numeric.split('.');
    let whole_part = parts.next().unwrap_or_default();
    let frac_part = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::OperationFailed);
    }

    if !whole_part.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(WalletError::OperationFailed);
    }

    let whole_sat = if whole_part.is_empty() {
        0i128
    } else {
        whole_part
            .parse::<i128>()
            .map_err(|_| WalletError::OperationFailed)?
            .checked_mul(SATOSHIS_PER_COIN)
            .ok_or(WalletError::OperationFailed)?
    };

    let mut frac_sat = 0i128;
    if let Some(frac) = frac_part {
        if !frac.chars().all(|ch| ch.is_ascii_digit()) || frac.len() > 8 {
            return Err(WalletError::OperationFailed);
        }
        if !frac.is_empty() {
            let padded = format!("{frac:0<8}");
            frac_sat = padded
                .parse::<i128>()
                .map_err(|_| WalletError::OperationFailed)?;
        }
    }

    let combined = whole_sat
        .checked_add(frac_sat)
        .ok_or(WalletError::OperationFailed)?;
    Ok(if is_negative { -combined } else { combined })
}

pub fn satoshis_to_decimal_string(value: i128) -> String {
    let is_negative = value.is_negative();
    let absolute = value.unsigned_abs();
    let whole = absolute / SATOSHIS_PER_COIN as u128;
    let frac = absolute % SATOSHIS_PER_COIN as u128;
    if is_negative {
        format!("-{whole}.{frac:08}")
    } else {
        format!("{whole}.{frac:08}")
    }
}

fn map_build_error(error: TxBuildError<Infallible>) -> WalletError {
    match error {
        TxBuildError::InsufficientFunds(_) | TxBuildError::ChangeRequired(_) => {
            WalletError::InsufficientFunds
        }
        _ => dlight_send_failure("transaction build", error.to_string()),
    }
}

fn parse_memo_bytes(memo: Option<&str>) -> Result<MemoBytes, WalletError> {
    let Some(value) = memo.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(MemoBytes::empty());
    };
    MemoBytes::from_bytes(value.as_bytes()).map_err(|error| {
        dlight_send_failure(
            "memo parsing",
            format!("memo is not valid for Sapling encoding: {error}"),
        )
    })
}

fn decode_shielded_delivery(
    address: &str,
    network: WalletNetwork,
) -> Result<sapling::PaymentAddress, WalletError> {
    let hrp = sapling_payment_address_hrp(network);
    zcash_client_backend::encoding::decode_payment_address(hrp, address.trim())
        .map_err(|_| WalletError::InvalidAddress)
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    zcash_protocol::constants::mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn decode_transparent_delivery(
    address: &str,
) -> Result<zcash_transparent::address::TransparentAddress, WalletError> {
    super::recipient_resolution::decode_r_address(address)
}

fn verus_send_params(network: WalletNetwork) -> VerusConsensusParams {
    let activation_height = match network {
        WalletNetwork::Mainnet => VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT,
        WalletNetwork::Testnet => VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT,
    };

    VerusConsensusParams {
        network_type: match network {
            WalletNetwork::Mainnet => NetworkType::Main,
            WalletNetwork::Testnet => NetworkType::Test,
        },
        sapling_activation_height: BlockHeight::from_u32(
            activation_height
                .try_into()
                .expect("known verus sapling activation heights fit in u32"),
        ),
    }
}

fn select_notes(notes: &[SpendableNote], required_sats: u64) -> Option<(Vec<SpendableNote>, u64)> {
    if required_sats == 0 {
        return Some((vec![], 0));
    }

    let mut selected = Vec::<SpendableNote>::new();
    let mut total = 0u64;
    for note in notes {
        selected.push(note.clone());
        total = total.saturating_add(note.value_sats);
        if total >= required_sats {
            return Some((selected, total));
        }
    }
    None
}

async fn broadcast_transaction(endpoint: &str, raw_tx: Vec<u8>) -> Result<(), WalletError> {
    let grpc_endpoint = normalize_grpc_endpoint(endpoint)?;
    let parsed_uri: Uri = grpc_endpoint
        .parse()
        .map_err(|_| WalletError::UnsupportedChannel)?;
    let host = parsed_uri.host().ok_or(WalletError::UnsupportedChannel)?;
    let is_https = parsed_uri.scheme_str() == Some("https");

    let endpoint_builder = Endpoint::from_shared(grpc_endpoint)
        .map_err(|_| WalletError::UnsupportedChannel)?
        .connect_timeout(Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS))
        .tcp_nodelay(true);

    let endpoint_builder = if is_https {
        endpoint_builder
            .tls_config(
                ClientTlsConfig::new()
                    .with_webpki_roots()
                    .domain_name(host.to_string()),
            )
            .map_err(|error| {
                dlight_send_failure(
                    "lightwalletd TLS setup",
                    format!("failed to configure TLS for broadcast endpoint: {error}"),
                )
            })?
    } else {
        endpoint_builder
    };

    let channel: Channel = endpoint_builder
        .connect()
        .await
        .map_err(|error| {
            dlight_send_failure(
                "lightwalletd connection",
                format!("failed to connect to broadcast endpoint: {error}"),
            )
        })?;
    let mut client = CompactTxStreamerClient::new(channel);

    let response = client
        .send_transaction(service::RawTransaction {
            data: raw_tx,
            height: 0,
        })
        .await
        .map_err(|error| {
            eprintln!(
                "[dlight_private][spend_send] broadcast rpc failed: {}",
                error
            );
            dlight_send_failure(
                "lightwalletd RPC",
                format!("sendTransaction RPC call failed: {error}"),
            )
        })?
        .into_inner();

    if response.error_code != 0 {
        let error_message = response.error_message.trim().to_string();
        eprintln!(
            "[dlight_private][spend_send] broadcast rejected code={} message={}",
            response.error_code, error_message
        );
        let message_lower = error_message.to_ascii_lowercase();
        if message_lower.contains("insufficient") {
            return Err(WalletError::InsufficientFunds);
        }
        let detail = if error_message.is_empty() {
            format!(
                "Broadcast was rejected by lightwalletd (code {}).",
                response.error_code
            )
        } else {
            format!(
                "Broadcast was rejected by lightwalletd (code {}): {}",
                response.error_code, error_message
            )
        };
        return Err(WalletError::DlightBroadcastRejected(detail));
    }

    Ok(())
}

fn dlight_send_failure(stage: &str, detail: impl Into<String>) -> WalletError {
    let detail = detail.into();
    let normalized = detail.split_whitespace().collect::<Vec<_>>().join(" ");
    let suffix = if normalized.is_empty() {
        "unknown error".to_string()
    } else {
        normalized
    };
    WalletError::DlightSendFailed(format!("dlight send failed during {stage}: {suffix}"))
}

#[cfg(test)]
mod tests {
    use crate::types::WalletError;

    use super::{parse_decimal_to_satoshis, resolve_send_value};

    #[test]
    fn parse_decimal_to_satoshis_supports_eight_decimals() {
        assert_eq!(
            parse_decimal_to_satoshis("12.34567890").expect("valid amount"),
            1_234_567_890
        );
        assert_eq!(parse_decimal_to_satoshis("0.00000001").expect("1 sat"), 1);
        assert_eq!(parse_decimal_to_satoshis("1").expect("whole"), 100_000_000);
    }

    #[test]
    fn parse_decimal_to_satoshis_rejects_precision_overflow() {
        assert!(matches!(
            parse_decimal_to_satoshis("1.123456789"),
            Err(WalletError::OperationFailed)
        ));
    }

    #[test]
    fn resolve_send_value_deducts_fee_for_max_send() {
        let (value_sats, fee_taken, message) =
            resolve_send_value(1_000_000, 1_000_000, 10_000).expect("max send should be adjusted");
        assert_eq!(value_sats, 990_000);
        assert!(fee_taken);
        assert!(message.is_some());
    }

    #[test]
    fn resolve_send_value_rejects_insufficient_balance() {
        assert!(matches!(
            resolve_send_value(100_000_000, 50_000_000, 10_000),
            Err(WalletError::InsufficientFunds)
        ));
    }
}
