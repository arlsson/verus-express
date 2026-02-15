//
// Reserve-transfer route descriptors.
// Final binary encoding and flag semantics are implemented in a later phase.

#[derive(Debug, Clone, Default)]
pub struct ReserveTransferRoute {
    pub convert_to: Option<String>,
    pub export_to: Option<String>,
    pub via: Option<String>,
    pub map_to: Option<String>,
    pub preconvert: bool,
}
