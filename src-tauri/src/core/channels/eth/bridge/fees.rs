//
// Fee quote structures for bridge preflight.

#[derive(Debug, Clone, Default)]
pub struct BridgeFeeQuote {
    pub network_fee_wei: String,
    pub bridge_fee_wei: String,
    pub import_fee_wei: Option<String>,
    pub total_max_fee_wei: String,
}
