//
// Bridge token mapping primitives used by conversion-path and preflight routines.

#[derive(Debug, Clone)]
pub struct BridgeTokenMapping {
    pub verus_currency_id: String,
    pub erc20_contract_address: String,
    pub launch_system_id: Option<String>,
}
