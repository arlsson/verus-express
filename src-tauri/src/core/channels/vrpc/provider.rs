//
// Module 5: VRPC HTTP JSON-RPC client. Allowlist-only endpoints; TTL cache; no sensitive data in logs.

use std::collections::HashMap;
use std::error::Error as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::Value;
use tokio::sync::{Mutex as AsyncMutex, Notify};

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Trusted VRPC allowlist (per backend architecture plan).
const VRPC_MAINNET: &str = "https://api.verus.services/";
const VRPC_TESTNET: &str = "https://api.verustest.net/";

/// TTL for cached responses (seconds).
const TTL_BALANCE: u64 = 5;
const TTL_DELTAS: u64 = 10;
const TTL_MEMPOOL: u64 = 10;
const TTL_UTXOS: u64 = 5;
const TTL_GETINFO: u64 = 1;
const TTL_CURRENCY: u64 = 60;
const TTL_LISTCURRENCIES: u64 = 600;
const READ_RETRY_ATTEMPTS: u8 = 3;
const READ_RETRY_BASE_DELAY_MS: u64 = 250;
const STALE_CACHE_MAX_AGE_SECS: u64 = 120;

fn params_getaddressbalance(addresses: &[String]) -> Result<Value, WalletError> {
    if addresses.is_empty() {
        return Err(WalletError::InvalidAddress);
    }
    Ok(serde_json::json!([{"addresses": addresses, "friendlynames": true}]))
}

fn params_getaddressdeltas(addresses: &[String]) -> Value {
    if addresses.is_empty() {
        serde_json::json!([{}])
    } else {
        serde_json::json!([{"addresses": addresses, "friendlynames": true, "verbosity": 1}])
    }
}

fn params_getaddressmempool(addresses: &[String]) -> Value {
    if addresses.is_empty() {
        serde_json::json!([{}])
    } else {
        serde_json::json!([{"addresses": addresses, "friendlynames": true, "verbosity": 1}])
    }
}

#[derive(Clone)]
struct CachedEntry {
    value: Value,
    expires_at: Instant,
}

/// HTTP JSON-RPC client for Verus daemon API. Endpoints are allowlist-only.
pub struct VrpcProvider {
    client: Client,
    base_url: String,
    cache: Mutex<HashMap<String, CachedEntry>>,
    in_flight: AsyncMutex<HashMap<String, Arc<Notify>>>,
}

impl VrpcProvider {
    fn build_http_client() -> Client {
        Client::builder()
            // Force a deterministic transport stack in-app:
            // rustls only, direct connection, and HTTP/1.1.
            .use_rustls_tls()
            .no_proxy()
            .http1_only()
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    pub fn new_mainnet() -> Self {
        Self {
            client: Self::build_http_client(),
            base_url: VRPC_MAINNET.to_string(),
            cache: Mutex::new(HashMap::new()),
            in_flight: AsyncMutex::new(HashMap::new()),
        }
    }

    pub fn new_testnet() -> Self {
        Self {
            client: Self::build_http_client(),
            base_url: VRPC_TESTNET.to_string(),
            cache: Mutex::new(HashMap::new()),
            in_flight: AsyncMutex::new(HashMap::new()),
        }
    }

    /// Default: mainnet.
    pub fn new() -> Self {
        Self::new_mainnet()
    }

    fn cache_key(method: &str, params: &Value) -> String {
        format!(
            "{}:{}",
            method,
            serde_json::to_string(params).unwrap_or_default()
        )
    }

    fn get_cached(&self, key: &str) -> Option<Value> {
        let cache = self.cache.lock().ok()?;
        let entry = cache.get(key)?;
        if entry.expires_at > Instant::now() {
            return Some(entry.value.clone());
        }
        None
    }

    fn get_stale_cached(&self, key: &str) -> Option<Value> {
        let cache = self.cache.lock().ok()?;
        let entry = cache.get(key)?;
        let now = Instant::now();

        if entry.expires_at > now {
            return Some(entry.value.clone());
        }

        let age = now.saturating_duration_since(entry.expires_at);
        if age <= Duration::from_secs(STALE_CACHE_MAX_AGE_SECS) {
            return Some(entry.value.clone());
        }
        None
    }

    fn set_cached(&self, key: String, value: Value, ttl_secs: u64) {
        if let Ok(mut cache) = self.cache.lock() {
            let now = Instant::now();
            let stale_limit = Duration::from_secs(STALE_CACHE_MAX_AGE_SECS);
            cache.retain(|_, entry| {
                if entry.expires_at > now {
                    return true;
                }
                now.saturating_duration_since(entry.expires_at) <= stale_limit
            });
            cache.insert(
                key,
                CachedEntry {
                    value,
                    expires_at: now + Duration::from_secs(ttl_secs),
                },
            );
        }
    }

    fn classify_network_error(err: &reqwest::Error) -> &'static str {
        let msg = err.to_string().to_ascii_lowercase();
        if err.is_timeout() {
            return "timeout";
        }
        if err.is_connect() {
            if msg.contains("dns")
                || msg.contains("lookup")
                || msg.contains("name or service not known")
            {
                return "dns";
            }
            return "connect";
        }
        if err.is_request() {
            return "request";
        }
        if err.is_decode() {
            return "decode";
        }
        if err.is_body() {
            return "body";
        }
        "network"
    }

    fn format_network_error_details(err: &reqwest::Error) -> String {
        let mut out = err.to_string();
        let mut source = err.source();
        while let Some(cause) = source {
            out.push_str(" | caused by: ");
            out.push_str(&cause.to_string());
            source = cause.source();
        }
        out
    }

    async fn fetch_result_with_retry(
        &self,
        method: &str,
        body: &Value,
        attempts: u8,
    ) -> Result<Value, WalletError> {
        for attempt in 0..attempts {
            let is_last_attempt = attempt + 1 >= attempts;
            let delay = Duration::from_millis(
                READ_RETRY_BASE_DELAY_MS.saturating_mul((attempt as u64) + 1),
            );

            let res = match self.client.post(&self.base_url).json(body).send().await {
                Ok(v) => v,
                Err(err) => {
                    let category = Self::classify_network_error(&err);
                    if !is_last_attempt {
                        println!(
                            "[VRPC] {} network failure ({}) on attempt {}/{}: {}",
                            method,
                            category,
                            attempt + 1,
                            attempts,
                            err
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    let details = Self::format_network_error_details(&err);
                    println!(
                        "[VRPC] {} network failure ({}) on final attempt {}/{}: {}",
                        method,
                        category,
                        attempt + 1,
                        attempts,
                        details
                    );
                    return Err(WalletError::NetworkError);
                }
            };

            if !res.status().is_success() {
                if !is_last_attempt && res.status().is_server_error() {
                    println!(
                        "[VRPC] {} HTTP {} on attempt {}/{}",
                        method,
                        res.status(),
                        attempt + 1,
                        attempts
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(WalletError::OperationFailed);
            }

            let json: Value = match res.json().await {
                Ok(v) => v,
                Err(err) => {
                    if !is_last_attempt {
                        println!(
                            "[VRPC] {} response parse failure on attempt {}/{}: {}",
                            method,
                            attempt + 1,
                            attempts,
                            err
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(WalletError::NetworkError);
                }
            };

            if let Some(error_obj) = json.get("error").filter(|e| !e.is_null()) {
                println!("[VRPC] {} RPC error response: {}", method, error_obj);
                return Err(WalletError::OperationFailed);
            }

            return json
                .get("result")
                .filter(|v| !v.is_null())
                .cloned()
                .ok_or(WalletError::OperationFailed);
        }

        Err(WalletError::NetworkError)
    }

    async fn call(&self, method: &str, params: Value, ttl_secs: u64) -> Result<Value, WalletError> {
        let key = Self::cache_key(method, &params);
        if let Some(cached) = self.get_cached(&key) {
            return Ok(cached);
        }

        let body = serde_json::json!({
            "jsonrpc": "1.0",
            "id": "lite-wallet",
            "method": method,
            "params": params
        });

        if ttl_secs == 0 {
            return self.fetch_result_with_retry(method, &body, 1).await;
        }

        let stale = self.get_stale_cached(&key);
        let notify = {
            let mut in_flight = self.in_flight.lock().await;
            if let Some(existing) = in_flight.get(&key) {
                Some(existing.clone())
            } else {
                in_flight.insert(key.clone(), Arc::new(Notify::new()));
                None
            }
        };

        if let Some(wait_for) = notify {
            wait_for.notified().await;
            if let Some(cached) = self.get_cached(&key) {
                return Ok(cached);
            }
            if let Some(stale_cached) = self.get_stale_cached(&key) {
                println!(
                    "[VRPC] {} using stale cached response after joined in-flight request failed",
                    method
                );
                return Ok(stale_cached);
            }
            return Err(WalletError::NetworkError);
        }

        let result = self
            .fetch_result_with_retry(method, &body, READ_RETRY_ATTEMPTS)
            .await;

        let waiters = {
            let mut in_flight = self.in_flight.lock().await;
            in_flight.remove(&key)
        };
        if let Some(waiters) = waiters {
            waiters.notify_waiters();
        }

        match result {
            Ok(value) => {
                self.set_cached(key, value.clone(), ttl_secs);
                Ok(value)
            }
            Err(err) => {
                if let Some(stale_cached) = stale {
                    println!(
                        "[VRPC] {} using stale cached response after retries exhausted",
                        method
                    );
                    return Ok(stale_cached);
                }
                Err(err)
            }
        }
    }

    /// getaddressbalance: params [{"addresses": ["R..."], "friendlynames": true}]
    pub async fn getaddressbalance(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressbalance(addresses)?;
        self.call("getaddressbalance", params, TTL_BALANCE).await
    }

    /// getaddressdeltas: params [{"addresses": ["R..."], "friendlynames": true, "verbosity": 1}]
    pub async fn getaddressdeltas(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressdeltas(addresses);
        self.call("getaddressdeltas", params, TTL_DELTAS).await
    }

    /// getaddressmempool: unconfirmed tx for address(es)
    pub async fn getaddressmempool(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = params_getaddressmempool(addresses);
        self.call("getaddressmempool", params, TTL_MEMPOOL).await
    }

    /// getaddressutxos: spendable UTXOs for funding
    pub async fn getaddressutxos(&self, addresses: &[String]) -> Result<Value, WalletError> {
        let params = if addresses.is_empty() {
            return Err(WalletError::InvalidAddress);
        } else if addresses.len() == 1 {
            serde_json::json!([{"addresses": [addresses[0]]}])
        } else {
            serde_json::json!([{"addresses": addresses}])
        };
        self.call("getaddressutxos", params, TTL_UTXOS).await
    }

    /// createrawtransaction: inputs (array), outputs (object address -> amount)
    pub async fn createrawtransaction(
        &self,
        inputs: &[Value],
        outputs: &Value,
    ) -> Result<Value, WalletError> {
        let params = serde_json::json!([inputs, outputs]);
        self.call("createrawtransaction", params, 0).await
    }

    /// fundrawtransaction: hex (string) -> funded hex + fee etc.
    pub async fn fundrawtransaction(&self, hex_tx: &str) -> Result<Value, WalletError> {
        let params = serde_json::json!([hex_tx]);
        self.call("fundrawtransaction", params, 0).await
    }

    /// sendrawtransaction: signed hex
    pub async fn sendrawtransaction(&self, signed_hex: &str) -> Result<Value, WalletError> {
        let params = serde_json::json!([signed_hex]);
        self.call("sendrawtransaction", params, 0).await
    }

    /// getinfo: chain sync status
    pub async fn getinfo(&self) -> Result<Value, WalletError> {
        self.call("getinfo", serde_json::json!([]), TTL_GETINFO)
            .await
    }
}

impl Default for VrpcProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared mainnet/testnet VRPC providers; select by active wallet network.
pub struct VrpcProviderPool {
    mainnet: VrpcProvider,
    testnet: VrpcProvider,
}

impl VrpcProviderPool {
    pub fn new() -> Self {
        Self {
            mainnet: VrpcProvider::new_mainnet(),
            testnet: VrpcProvider::new_testnet(),
        }
    }

    pub fn for_network(&self, network: WalletNetwork) -> &VrpcProvider {
        match network {
            WalletNetwork::Mainnet => &self.mainnet,
            WalletNetwork::Testnet => &self.testnet,
        }
    }
}

impl Default for VrpcProviderPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getaddressbalance_params_are_object_form() {
        let addresses = vec!["RExampleAddress".to_string()];
        let params = params_getaddressbalance(&addresses).expect("params");
        assert_eq!(
            params,
            serde_json::json!([{"addresses": ["RExampleAddress"], "friendlynames": true}])
        );
    }

    #[test]
    fn getaddressdeltas_and_mempool_include_verbosity_and_friendlynames() {
        let addresses = vec!["RExampleAddress".to_string()];
        let deltas = params_getaddressdeltas(&addresses);
        let mempool = params_getaddressmempool(&addresses);
        let expected = serde_json::json!([{"addresses": ["RExampleAddress"], "friendlynames": true, "verbosity": 1}]);
        assert_eq!(deltas, expected);
        assert_eq!(mempool, expected);
    }
}
