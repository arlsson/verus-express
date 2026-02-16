use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::core::address_book::manager;
use crate::core::auth::SessionManager;
use crate::core::StrongholdStore;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    AddressBookContact, AddressBookSnapshot, SaveAddressBookContactRequest,
    ValidateDestinationAddressRequest, ValidateDestinationAddressResult, WalletError,
};

struct AddressBookContext {
    account_id: String,
    network: WalletNetwork,
    password_hash: Zeroizing<Vec<u8>>,
    stronghold_store: StrongholdStore,
}

async fn address_book_context(
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<AddressBookContext, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    Ok(AddressBookContext {
        account_id,
        network,
        password_hash,
        stronghold_store,
    })
}

async fn load_snapshot(context: &AddressBookContext) -> Result<AddressBookSnapshot, WalletError> {
    let payload = context
        .stronghold_store
        .load_address_book(&context.account_id, context.password_hash.as_ref())
        .await?;

    match payload {
        Some(raw) => serde_json::from_slice::<AddressBookSnapshot>(&raw)
            .map_err(|_| WalletError::OperationFailed),
        None => Ok(manager::empty_snapshot()),
    }
}

async fn store_snapshot(
    context: &AddressBookContext,
    snapshot: &AddressBookSnapshot,
) -> Result<(), WalletError> {
    let payload = serde_json::to_vec(snapshot).map_err(|_| WalletError::OperationFailed)?;
    context
        .stronghold_store
        .store_address_book(
            &context.account_id,
            context.password_hash.as_ref(),
            &payload,
        )
        .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_address_book_contacts(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<AddressBookContact>, WalletError> {
    let context = address_book_context(session_manager.inner()).await?;
    let snapshot = load_snapshot(&context).await?;
    Ok(manager::sorted_contacts(&snapshot))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_address_book_contact(
    request: SaveAddressBookContactRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<AddressBookContact, WalletError> {
    let context = address_book_context(session_manager.inner()).await?;
    let mut snapshot = load_snapshot(&context).await?;
    let saved = manager::upsert_contact(&mut snapshot, request, context.network)?;
    store_snapshot(&context, &snapshot).await?;
    Ok(saved)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_address_book_contact(
    contact_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let context = address_book_context(session_manager.inner()).await?;
    let mut snapshot = load_snapshot(&context).await?;
    let deleted = manager::delete_contact(&mut snapshot, &contact_id);
    if deleted {
        store_snapshot(&context, &snapshot).await?;
    }
    Ok(deleted)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mark_address_book_endpoint_used(
    endpoint_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let context = address_book_context(session_manager.inner()).await?;
    let mut snapshot = load_snapshot(&context).await?;
    let updated = manager::mark_endpoint_used(&mut snapshot, &endpoint_id);
    if updated {
        store_snapshot(&context, &snapshot).await?;
    }
    Ok(updated)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn validate_destination_address(
    request: ValidateDestinationAddressRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<ValidateDestinationAddressResult, WalletError> {
    let context = address_book_context(session_manager.inner()).await?;
    let normalized = manager::normalize_destination_address(
        request.kind.clone(),
        &request.address,
        context.network,
    );

    match normalized {
        Ok(value) => Ok(ValidateDestinationAddressResult {
            valid: true,
            normalized_address: Some(value),
            reason: None,
        }),
        Err(
            WalletError::InvalidAddress
            | WalletError::AddressBookInvalidInput
            | WalletError::AddressBookDuplicate
            | WalletError::AddressBookContactNotFound,
        ) => Ok(ValidateDestinationAddressResult {
            valid: false,
            normalized_address: None,
            reason: Some("invalid_destination".to_string()),
        }),
        Err(error) => Err(error),
    }
}
