use blake2b_simd::Params as Blake2bParams;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const ENV_REQUIRE_PARAMS: &str = "LITE_WALLET_REQUIRE_ZCASH_PARAMS";
const ENV_SPEND_SHA256: &str = "LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256";
const ENV_OUTPUT_SHA256: &str = "LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256";

const SAPLING_SPEND_BLAKE2B_DEFAULT: &str = "8270785a1a0d0bc77196f000ee6d221c9c9894f55307bd9357c3f0105d31ca63991ab91324160d8f53e2bbd3c2633a6eb8bdf5205d822e7f3f73edac51b2b70c";
const SAPLING_OUTPUT_BLAKE2B_DEFAULT: &str = "657e3d38dbb5cb5e7dd2970e8b03d69b4787dd907285b5a7f0790dcc8072f60bf593b32cc2d1c030e00ff5ae64bf84c5c3beb84ddc841d48264b4a171744d028";

const MIN_SPEND_PARAM_SIZE_BYTES: u64 = 40_000_000;
const MIN_OUTPUT_PARAM_SIZE_BYTES: u64 = 3_000_000;

fn main() {
    enforce_params_bundle_contract();
    tauri_build::build()
}

fn enforce_params_bundle_contract() {
    if !enforce_params_requested() {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let params_dir = PathBuf::from(manifest_dir).join("resources/zcash-params");

    verify_param_file(
        &params_dir,
        "sapling-spend.params",
        ENV_SPEND_SHA256,
        SAPLING_SPEND_BLAKE2B_DEFAULT,
        MIN_SPEND_PARAM_SIZE_BYTES,
    );
    verify_param_file(
        &params_dir,
        "sapling-output.params",
        ENV_OUTPUT_SHA256,
        SAPLING_OUTPUT_BLAKE2B_DEFAULT,
        MIN_OUTPUT_PARAM_SIZE_BYTES,
    );
}

fn enforce_params_requested() -> bool {
    std::env::var(ENV_REQUIRE_PARAMS)
        .ok()
        .map(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
enum ChecksumAlgorithm {
    Sha256,
    Blake2b512,
}

fn verify_param_file(
    params_dir: &PathBuf,
    file_name: &str,
    checksum_env: &str,
    default_checksum: &str,
    min_size_bytes: u64,
) {
    let file_path = params_dir.join(file_name);
    if !file_path.exists() {
        panic!(
            "Missing required Sapling params file: {}",
            file_path.display()
        );
    }

    let size = std::fs::metadata(&file_path)
        .unwrap_or_else(|err| {
            panic!(
                "Failed reading metadata for {}: {}",
                file_path.display(),
                err
            )
        })
        .len();
    if size < min_size_bytes {
        panic!(
            "Sapling params file {} appears to be a placeholder ({} bytes, expected at least {}).",
            file_path.display(),
            size,
            min_size_bytes
        );
    }

    let expected = std::env::var(checksum_env)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_checksum.to_string());

    let algorithm = match checksum_algorithm(&expected) {
        Some(value) => value,
        None => panic!(
            "Checksum format for {} is invalid. Expected 64-char sha256 or 128-char blake2b-512.",
            checksum_env
        ),
    };

    let bytes = std::fs::read(&file_path).unwrap_or_else(|err| {
        panic!(
            "Failed reading Sapling params file {}: {}",
            file_path.display(),
            err
        )
    });

    let actual = match algorithm {
        ChecksumAlgorithm::Sha256 => hex::encode(Sha256::digest(bytes)),
        ChecksumAlgorithm::Blake2b512 => Blake2bParams::new()
            .hash_length(64)
            .hash(&bytes)
            .to_hex()
            .to_string(),
    };

    if actual != expected {
        panic!(
            "Sapling params checksum mismatch for {}: expected {}, got {}",
            file_path.display(),
            expected,
            actual
        );
    }
}

fn checksum_algorithm(value: &str) -> Option<ChecksumAlgorithm> {
    if !value.chars().all(|char| char.is_ascii_hexdigit()) {
        return None;
    }

    match value.len() {
        64 => Some(ChecksumAlgorithm::Sha256),
        128 => Some(ChecksumAlgorithm::Blake2b512),
        _ => None,
    }
}
