use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AddressEndpointKind {
    Vrpc,
    Btc,
    Eth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressBookEndpoint {
    pub id: String,
    pub kind: AddressEndpointKind,
    pub address: String,
    pub normalized_address: String,
    pub label: String,
    pub last_used_at: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressBookContact {
    pub id: String,
    pub display_name: String,
    pub note: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub endpoints: Vec<AddressBookEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressBookSnapshot {
    pub schema_version: u8,
    pub contacts: Vec<AddressBookContact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAddressBookEndpointInput {
    pub id: Option<String>,
    pub kind: AddressEndpointKind,
    pub address: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAddressBookContactRequest {
    pub id: Option<String>,
    pub display_name: String,
    pub note: Option<String>,
    pub endpoints: Vec<SaveAddressBookEndpointInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateDestinationAddressRequest {
    pub kind: AddressEndpointKind,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateDestinationAddressResult {
    pub valid: bool,
    pub normalized_address: Option<String>,
    pub reason: Option<String>,
}
