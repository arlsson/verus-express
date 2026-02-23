use ethers::types::{Address, Bytes};

use super::delegator::{CcurrencyValueMap, CreserveTransfer, CtransferDestination};
use crate::types::WalletError;

pub const DEST_PKH: u8 = 2;
pub const DEST_ID: u8 = 4;
pub const DEST_ETH: u8 = 9;
pub const FLAG_DEST_AUX: u8 = 64;
pub const FLAG_DEST_GATEWAY: u8 = 128;

pub const RESERVE_TRANSFER_VALID: u32 = 1;
pub const RESERVE_TRANSFER_CONVERT: u32 = 2;
pub const RESERVE_TRANSFER_IMPORT_TO_SOURCE: u32 = 0x200;
pub const RESERVE_TRANSFER_RESERVE_TO_RESERVE: u32 = 0x400;

const I_ADDRESS_VERSION: u8 = 102;
const R_ADDRESS_VERSION: u8 = 60;

#[derive(Debug, Clone)]
pub struct ReserveTransferDestination {
    pub destination_type: u8,
    pub destination_address: Bytes,
    pub normalized_destination: String,
    pub is_eth_destination: bool,
    pub is_gateway_destination: bool,
}

#[derive(Debug, Clone)]
pub struct ReserveTransferPayload {
    pub version: u32,
    pub currency_value_currency: Address,
    pub currency_value_amount: u64,
    pub flags: u32,
    pub fee_currency_id: Address,
    pub fees: u64,
    pub destination_type: u8,
    pub destination_address: Bytes,
    pub dest_currency_id: Address,
    pub dest_system_id: Address,
    pub second_reserve_id: Address,
}

impl ReserveTransferPayload {
    pub fn into_contract_struct(self) -> CreserveTransfer {
        CreserveTransfer {
            version: self.version,
            currencyvalue: CcurrencyValueMap {
                currency: self.currency_value_currency,
                amount: self.currency_value_amount,
            },
            flags: self.flags,
            feecurrencyid: self.fee_currency_id,
            fees: self.fees,
            destination: CtransferDestination {
                destinationtype: self.destination_type,
                destinationaddress: self.destination_address,
            },
            destcurrencyid: self.dest_currency_id,
            destsystemid: self.dest_system_id,
            secondreserveid: self.second_reserve_id,
        }
    }
}

pub fn decode_base58_address(address: &str) -> Result<(u8, [u8; 20]), WalletError> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;
    if decoded.len() != 21 {
        return Err(WalletError::InvalidAddress);
    }

    let mut hash = [0u8; 20];
    hash.copy_from_slice(&decoded[1..]);
    Ok((decoded[0], hash))
}

pub fn to_eth_address_from_iaddress(i_address: &str) -> Result<Address, WalletError> {
    let (version, hash) = decode_base58_address(i_address)?;
    if version != I_ADDRESS_VERSION {
        return Err(WalletError::InvalidAddress);
    }
    Ok(Address::from(hash))
}

pub fn normalize_verus_destination(input: &str) -> Result<ReserveTransferDestination, WalletError> {
    let trimmed = input.trim();
    let (version, hash) = decode_base58_address(trimmed)?;
    let destination_type = match version {
        R_ADDRESS_VERSION => DEST_PKH,
        I_ADDRESS_VERSION => DEST_ID,
        _ => return Err(WalletError::InvalidAddress),
    };

    Ok(ReserveTransferDestination {
        destination_type,
        destination_address: Bytes::from(hash.to_vec()),
        normalized_destination: trimmed.to_string(),
        is_eth_destination: false,
        is_gateway_destination: false,
    })
}

pub fn normalize_eth_destination(input: &str) -> Result<(Address, String), WalletError> {
    let parsed = input
        .trim()
        .parse::<Address>()
        .map_err(|_| WalletError::InvalidAddress)?;
    Ok((parsed, format!("{:#x}", parsed)))
}

pub fn build_gateway_eth_destination(
    eth_destination: Address,
    gateway_iaddress: &str,
    import_fee_sats: u64,
    refund_vrpc_address: &str,
) -> Result<ReserveTransferDestination, WalletError> {
    let (refund_version, refund_hash) = decode_base58_address(refund_vrpc_address)?;
    let refund_type = match refund_version {
        R_ADDRESS_VERSION => DEST_PKH,
        I_ADDRESS_VERSION => DEST_ID,
        _ => return Err(WalletError::InvalidAddress),
    };

    let (_, gateway_hash) = decode_base58_address(gateway_iaddress)?;
    let gateway_code = [0u8; 20];
    let destination_type = DEST_ETH | FLAG_DEST_GATEWAY | FLAG_DEST_AUX;

    let empty_gateway_code = [0u8; 20];
    let aux_destination = serialize_transfer_destination(
        refund_type,
        &refund_hash,
        None,
        &empty_gateway_code,
        0,
        &[],
    )?;
    let serialized = serialize_transfer_destination(
        destination_type,
        eth_destination.as_bytes(),
        Some(gateway_hash),
        &gateway_code,
        import_fee_sats as i64,
        &[aux_destination],
    )?;

    // Verus delegator expects destinationtype separately and destinationaddress without
    // the type byte + compact-size length prefix for destination bytes (20 bytes => 2 bytes).
    if serialized.len() < 3 {
        return Err(WalletError::OperationFailed);
    }

    Ok(ReserveTransferDestination {
        destination_type,
        destination_address: Bytes::from(serialized[2..].to_vec()),
        normalized_destination: format!("{:#x}", eth_destination),
        is_eth_destination: true,
        is_gateway_destination: true,
    })
}

fn serialize_transfer_destination(
    destination_type: u8,
    destination_bytes: &[u8],
    gateway_id: Option<[u8; 20]>,
    gateway_code: &[u8; 20],
    fees_sats: i64,
    aux_destinations: &[Vec<u8>],
) -> Result<Vec<u8>, WalletError> {
    let mut buffer = Vec::<u8>::new();
    buffer.push(destination_type);
    write_var_slice(&mut buffer, destination_bytes);

    let has_gateway = (destination_type & FLAG_DEST_GATEWAY) != 0;
    let has_aux = (destination_type & FLAG_DEST_AUX) != 0;

    if has_gateway {
        let Some(gateway_hash) = gateway_id else {
            return Err(WalletError::OperationFailed);
        };
        buffer.extend_from_slice(&gateway_hash);
        buffer.extend_from_slice(gateway_code);
        buffer.extend_from_slice(&fees_sats.to_le_bytes());
    }

    if has_aux {
        write_compact_size(&mut buffer, aux_destinations.len() as u64);
        for aux in aux_destinations {
            write_var_slice(&mut buffer, aux);
        }
    }

    Ok(buffer)
}

fn write_var_slice(buffer: &mut Vec<u8>, value: &[u8]) {
    write_compact_size(buffer, value.len() as u64);
    buffer.extend_from_slice(value);
}

fn write_compact_size(buffer: &mut Vec<u8>, value: u64) {
    if value < 253 {
        buffer.push(value as u8);
    } else if value <= 0xFFFF {
        buffer.push(253);
        buffer.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value <= 0xFFFF_FFFF {
        buffer.push(254);
        buffer.extend_from_slice(&(value as u32).to_le_bytes());
    } else {
        buffer.push(255);
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_gateway_eth_destination, decode_base58_address, normalize_eth_destination,
        normalize_verus_destination, DEST_ETH, FLAG_DEST_AUX, FLAG_DEST_GATEWAY,
    };

    #[test]
    fn normalize_eth_destination_accepts_valid_address() {
        let (_, normalized) =
            normalize_eth_destination("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").expect("eth");
        assert_eq!(normalized, "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    }

    #[test]
    fn normalize_verus_destination_accepts_r_and_i_addresses() {
        let r = normalize_verus_destination("RLcoqsCLBQJPvciM1EvFzXH9p42Y61AtiB");
        let i = normalize_verus_destination("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV");
        assert!(r.is_ok());
        assert!(i.is_ok());
    }

    #[test]
    fn build_gateway_eth_destination_sets_gateway_type_and_payload() {
        let (eth_address, _) =
            normalize_eth_destination("0x0000000000000000000000000000000000000001").expect("eth");
        let destination = build_gateway_eth_destination(
            eth_address,
            "i3f7tSctFkiPpiedY8QR5Tep9p4qDVebDx",
            1_000_000,
            "RLcoqsCLBQJPvciM1EvFzXH9p42Y61AtiB",
        )
        .expect("gateway destination");
        assert_eq!(
            destination.destination_type,
            DEST_ETH | FLAG_DEST_GATEWAY | FLAG_DEST_AUX
        );
        assert!(!destination.destination_address.is_empty());
        assert!(destination.is_gateway_destination);
        assert!(destination.is_eth_destination);
    }

    #[test]
    fn decode_base58_address_rejects_invalid_input() {
        assert!(decode_base58_address("not-an-address").is_err());
    }
}
