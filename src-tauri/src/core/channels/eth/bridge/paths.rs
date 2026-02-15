//
// Bridge conversion-path discovery.
// Phase-1 scope: VRPC conversion path normalization for desktop bridge wizard.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::core::channels::vrpc::VrpcProvider;
use crate::types::bridge::{
    BridgeConversionPathQuote, BridgeConversionPathRequest, BridgeConversionPathsResult,
};
use crate::types::WalletError;

const IS_FRACTIONAL_FLAG: u64 = 0x01;
const IS_GATEWAY_FLAG: u64 = 0x80;
const IS_GATEWAY_CONVERTER_FLAG: u64 = 0x200;

#[derive(Debug, Clone)]
struct CurrencyNode {
    currency_id: String,
    fully_qualified_name: Option<String>,
    name: Option<String>,
    symbol: Option<String>,
    options: u64,
    system_id: Option<String>,
    parent: Option<String>,
    launch_system_id: Option<String>,
    currencies: Vec<String>,
    best_prices: HashMap<String, f64>,
}

impl CurrencyNode {
    fn display_name(&self) -> Option<String> {
        self.fully_qualified_name
            .clone()
            .or_else(|| self.name.clone())
    }

    fn display_ticker(&self) -> Option<String> {
        self.symbol
            .clone()
            .or_else(|| self.name.clone())
            .or_else(|| self.display_name())
    }

    fn reference(&self) -> String {
        self.fully_qualified_name
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| self.currency_id.clone())
    }

    fn matches_filter(&self, filter: &str) -> bool {
        equals_ignore_case(&self.currency_id, filter)
            || self
                .fully_qualified_name
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
            || self
                .name
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
            || self
                .symbol
                .as_deref()
                .map(|value| equals_ignore_case(value, filter))
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
struct RouteCandidate {
    destination_id: String,
    via_id: Option<String>,
    export_to_id: Option<String>,
    price: Option<f64>,
    via_price_in_root: Option<f64>,
    dest_price_in_via: Option<f64>,
    gateway: bool,
}

pub async fn get_conversion_paths(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
) -> Result<BridgeConversionPathsResult, WalletError> {
    let source_currency = request.source_currency.trim();
    if source_currency.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let source_definition = vrpc_provider.getcurrency(source_currency).await?;
    let destination_definition = match request
        .destination_currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(destination_currency) => Some(vrpc_provider.getcurrency(destination_currency).await?),
        None => None,
    };

    let destination_currency_filter = destination_definition
        .as_ref()
        .and_then(extract_currency_id);
    let chain_currency_id = request
        .channel_id
        .rsplit_once('.')
        .map(|(_, system_id)| system_id.to_string());

    let normalized_paths = match vrpc_provider
        .getcurrencyconversionpaths(&source_definition, destination_definition.as_ref())
        .await
    {
        Ok(raw_paths) => parse_conversion_paths(raw_paths)?,
        Err(WalletError::BridgeNotImplemented) => {
            println!(
                    "[BRIDGE] VRPC conversion-path RPC unavailable; deriving conversion graph from listcurrencies"
                );
            derive_paths_from_listcurrencies(
                vrpc_provider,
                &source_definition,
                destination_currency_filter.as_deref(),
                chain_currency_id.as_deref(),
            )
            .await?
        }
        Err(err) => return Err(err),
    };

    Ok(BridgeConversionPathsResult {
        source_currency: request.source_currency.clone(),
        paths: normalized_paths,
    })
}

async fn derive_paths_from_listcurrencies(
    vrpc_provider: &VrpcProvider,
    source_definition: &Value,
    destination_currency_filter: Option<&str>,
    chain_currency_id: Option<&str>,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let mut list_payloads = Vec::new();
    let mut used_partitioned_list = true;

    for systemtype in ["local", "pbaas", "imported"] {
        match vrpc_provider
            .listcurrencies_with_systemtype(systemtype)
            .await
        {
            Ok(payload) => list_payloads.push(payload),
            Err(err) => {
                println!(
                    "[BRIDGE] listcurrencies systemtype={} unavailable in fallback: {:?}",
                    systemtype, err
                );
                used_partitioned_list = false;
                break;
            }
        }
    }

    if !used_partitioned_list || list_payloads.is_empty() {
        list_payloads.clear();
        list_payloads.push(vrpc_provider.listcurrencies().await?);
    }

    let chain_definition = if let Some(chain_id) = chain_currency_id {
        match vrpc_provider.getcurrency(chain_id).await {
            Ok(definition) => Some(definition),
            Err(err) => {
                println!(
                    "[BRIDGE] chain currency lookup failed for {} during fallback: {:?}",
                    chain_id, err
                );
                None
            }
        }
    } else {
        None
    };

    let payload_refs = list_payloads.iter().collect::<Vec<_>>();
    derive_paths_from_list_payloads(
        &payload_refs,
        source_definition,
        destination_currency_filter,
        chain_currency_id,
        chain_definition.as_ref(),
    )
}

fn derive_paths_from_list_payloads(
    list_payloads: &[&Value],
    source_definition: &Value,
    destination_currency_filter: Option<&str>,
    chain_currency_id: Option<&str>,
    chain_definition: Option<&Value>,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let mut currencies: HashMap<String, CurrencyNode> = HashMap::new();
    for payload in list_payloads {
        merge_currency_nodes_from_list_payload(payload, &mut currencies)?;
    }

    let source_currency = currency_node_from_definition(
        source_definition,
        source_definition.get("bestcurrencystate"),
    )
    .ok_or(WalletError::OperationFailed)?;
    currencies
        .entry(source_currency.currency_id.clone())
        .or_insert_with(|| source_currency.clone());

    let chain_currency = chain_definition.and_then(|definition| {
        currency_node_from_definition(definition, definition.get("bestcurrencystate"))
    });
    if let Some(chain_currency) = chain_currency.as_ref() {
        currencies
            .entry(chain_currency.currency_id.clone())
            .or_insert_with(|| chain_currency.clone());
    }

    let effective_chain_currency_id = chain_currency_id
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            chain_currency
                .as_ref()
                .map(|value| value.currency_id.clone())
        })
        .or_else(|| source_currency.system_id.clone())
        .unwrap_or_else(|| source_currency.currency_id.clone());

    let chain_currency_context = currencies
        .get(&effective_chain_currency_id)
        .cloned()
        .or(chain_currency);

    let mut routes = collect_routes_for_source(
        &currencies,
        &source_currency,
        &effective_chain_currency_id,
        chain_currency_context.as_ref(),
        true,
        &HashSet::new(),
        None,
        None,
    );

    if let Some(filter) = destination_currency_filter {
        routes.retain(|destination_id, _| {
            currencies
                .get(destination_id)
                .map(|currency| currency.matches_filter(filter))
                .unwrap_or_else(|| equals_ignore_case(destination_id, filter))
        });
    }

    Ok(routes_to_quotes(&routes, &currencies))
}

fn collect_routes_for_source(
    currencies: &HashMap<String, CurrencyNode>,
    source: &CurrencyNode,
    chain_currency_id: &str,
    chain_currency: Option<&CurrencyNode>,
    include_via: bool,
    ignore_currencies: &HashSet<String>,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
) -> HashMap<String, Vec<RouteCandidate>> {
    let mut routes: HashMap<String, Vec<RouteCandidate>> = HashMap::new();

    for destination in currencies.values() {
        if !contains_currency(destination, &source.currency_id) {
            continue;
        }

        let Some((price, via_price_in_root, dest_price_in_via)) =
            compute_destination_price(source, destination, via, root)
        else {
            continue;
        };

        let gateway = is_gateway(destination);
        let gateway_converter = is_gateway_converter(destination)
            || is_gateway_converter(source)
            || via.map(is_gateway_converter).unwrap_or(false);
        let fractional_converter = via.unwrap_or(destination);

        let export_to_id = if gateway || gateway_converter {
            if gateway {
                Some(destination.currency_id.clone())
            } else {
                resolve_export_to_currency_id(fractional_converter, chain_currency_id)
            }
        } else {
            None
        };

        add_route_candidate(
            &mut routes,
            RouteCandidate {
                destination_id: destination.currency_id.clone(),
                via_id: via.map(|currency| currency.currency_id.clone()),
                export_to_id: export_to_id.clone(),
                price: Some(price),
                via_price_in_root,
                dest_price_in_via,
                gateway,
            },
        );

        if gateway || gateway_converter {
            add_route_candidate(
                &mut routes,
                RouteCandidate {
                    destination_id: destination.currency_id.clone(),
                    via_id: via.map(|currency| currency.currency_id.clone()),
                    export_to_id: None,
                    price: Some(price),
                    via_price_in_root,
                    dest_price_in_via,
                    gateway: false,
                },
            );
        }
    }

    let fractional_source = is_fractional_source(source, chain_currency_id);
    if fractional_source {
        for reserve_currency_id in &source.currencies {
            if contains_ignore(ignore_currencies, reserve_currency_id) {
                continue;
            }

            let Some(destination) = currencies.get(reserve_currency_id) else {
                continue;
            };

            let Some((price, via_price_in_root, dest_price_in_via)) =
                compute_reserve_price(source, reserve_currency_id, via, root)
            else {
                continue;
            };

            let gateway = is_gateway(destination);
            let gateway_converter = is_gateway_converter(destination)
                || is_gateway_converter(source)
                || via.map(is_gateway_converter).unwrap_or(false);
            let fractional_converter = via.unwrap_or(source);

            let export_to_id = if gateway || gateway_converter {
                if gateway {
                    Some(destination.currency_id.clone())
                } else {
                    resolve_export_to_currency_id(fractional_converter, chain_currency_id)
                }
            } else {
                None
            };

            add_route_candidate(
                &mut routes,
                RouteCandidate {
                    destination_id: destination.currency_id.clone(),
                    via_id: via.map(|currency| currency.currency_id.clone()),
                    export_to_id: export_to_id.clone(),
                    price: Some(price),
                    via_price_in_root,
                    dest_price_in_via,
                    gateway,
                },
            );

            if gateway || gateway_converter {
                add_route_candidate(
                    &mut routes,
                    RouteCandidate {
                        destination_id: destination.currency_id.clone(),
                        via_id: via.map(|currency| currency.currency_id.clone()),
                        export_to_id: None,
                        price: Some(price),
                        via_price_in_root,
                        dest_price_in_via,
                        gateway: false,
                    },
                );
            }
        }
    }

    apply_visibility_filters(
        &mut routes,
        currencies,
        source,
        chain_currency_id,
        chain_currency,
    );

    routes.retain(|destination_id, candidates| {
        !equals_ignore_case(destination_id, &source.currency_id) && !candidates.is_empty()
    });

    if include_via {
        let candidate_destinations = routes.keys().cloned().collect::<Vec<_>>();
        for destination_id in candidate_destinations {
            let Some(destination_currency) = currencies.get(&destination_id) else {
                continue;
            };

            if !is_fractional(destination_currency)
                || !contains_currency(destination_currency, &source.currency_id)
                || contains_ignore(ignore_currencies, &destination_currency.currency_id)
            {
                continue;
            }

            let mut next_ignore = ignore_currencies.clone();
            next_ignore.insert(source.currency_id.clone());

            let via_routes = collect_routes_for_source(
                currencies,
                destination_currency,
                chain_currency_id,
                chain_currency,
                false,
                &next_ignore,
                Some(destination_currency),
                Some(source),
            );
            merge_route_maps(&mut routes, via_routes);
        }
    }

    routes
}

fn merge_route_maps(
    destination: &mut HashMap<String, Vec<RouteCandidate>>,
    source: HashMap<String, Vec<RouteCandidate>>,
) {
    for candidates in source.into_values() {
        for candidate in candidates {
            add_route_candidate(destination, candidate);
        }
    }
}

fn add_route_candidate(
    routes: &mut HashMap<String, Vec<RouteCandidate>>,
    candidate: RouteCandidate,
) {
    let entry = routes.entry(candidate.destination_id.clone()).or_default();
    if entry
        .iter()
        .any(|existing| route_identity_matches(existing, &candidate))
    {
        return;
    }
    entry.push(candidate);
}

fn route_identity_matches(left: &RouteCandidate, right: &RouteCandidate) -> bool {
    equals_ignore_case(&left.destination_id, &right.destination_id)
        && option_equals_ignore_case(left.via_id.as_deref(), right.via_id.as_deref())
        && option_equals_ignore_case(left.export_to_id.as_deref(), right.export_to_id.as_deref())
}

fn option_equals_ignore_case(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(l), Some(r)) => equals_ignore_case(l, r),
        (None, None) => true,
        _ => false,
    }
}

fn apply_visibility_filters(
    routes: &mut HashMap<String, Vec<RouteCandidate>>,
    currencies: &HashMap<String, CurrencyNode>,
    source: &CurrencyNode,
    chain_currency_id: &str,
    chain_currency: Option<&CurrencyNode>,
) {
    let fractional_source = is_fractional_source(source, chain_currency_id);

    for (destination_id, candidates) in routes.iter_mut() {
        let Some(destination) = currencies.get(destination_id) else {
            candidates.clear();
            continue;
        };

        candidates.retain(|candidate| {
            let via_currency = candidate
                .via_id
                .as_deref()
                .and_then(|currency_id| currencies.get(currency_id));

            let fractional_converter = if let Some(via_currency) = via_currency {
                Some(via_currency)
            } else {
                let fractional_destination = is_fractional(destination);
                if fractional_source && !fractional_destination {
                    Some(source)
                } else if !fractional_source && fractional_destination {
                    Some(destination)
                } else if !source.currencies.is_empty() && !destination.currencies.is_empty() {
                    if contains_currency(source, &destination.currency_id) {
                        Some(source)
                    } else {
                        Some(destination)
                    }
                } else {
                    None
                }
            };

            let Some(fractional_converter) = fractional_converter else {
                // If we cannot infer a fractional converter context, keep the route.
                // This mirrors a permissive fallback so we do not hide valid direct routes.
                return true;
            };

            if let Some(fractional_system_id) = fractional_converter.system_id.as_deref() {
                if !equals_ignore_case(fractional_system_id, chain_currency_id) {
                    let export_matches = candidate
                        .export_to_id
                        .as_deref()
                        .map(|export_to_id| equals_ignore_case(export_to_id, fractional_system_id))
                        .unwrap_or(false);
                    if !export_matches {
                        return false;
                    }
                }
            }

            let Some(export_to_id) = candidate.export_to_id.as_deref() else {
                return true;
            };
            let Some(chain_currency) = chain_currency else {
                return true;
            };
            let Some(export_to_currency) = currencies.get(export_to_id) else {
                return true;
            };

            let see_by_launch = export_to_currency
                .launch_system_id
                .as_deref()
                .map(|value| equals_ignore_case(value, &chain_currency.currency_id))
                .unwrap_or(false)
                || chain_currency
                    .launch_system_id
                    .as_deref()
                    .map(|value| equals_ignore_case(value, &export_to_currency.currency_id))
                    .unwrap_or(false);

            let see_by_parent = export_to_currency
                .parent
                .as_deref()
                .map(|value| equals_ignore_case(value, &chain_currency.currency_id))
                .unwrap_or(false)
                || chain_currency
                    .parent
                    .as_deref()
                    .map(|value| equals_ignore_case(value, &export_to_currency.currency_id))
                    .unwrap_or(false);

            see_by_launch || see_by_parent
        });
    }
}

fn compute_destination_price(
    source: &CurrencyNode,
    destination: &CurrencyNode,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
) -> Option<(f64, Option<f64>, Option<f64>)> {
    if let Some(via_currency) = via {
        let root_currency = root?;
        let via_price_raw = via_currency
            .best_prices
            .get(&root_currency.currency_id)
            .copied()?;
        let dest_price_raw = via_currency
            .best_prices
            .get(&destination.currency_id)
            .copied()?;

        let via_price_in_root = normalize_positive(1.0 / via_price_raw)?;
        let dest_price_in_via = normalize_positive(dest_price_raw)?;
        let price = normalize_positive(via_price_in_root * dest_price_in_via)?;
        return Some((price, Some(via_price_in_root), Some(dest_price_in_via)));
    }

    let source_price_raw = destination.best_prices.get(&source.currency_id).copied()?;
    let price = normalize_positive(1.0 / source_price_raw)?;
    Some((price, None, None))
}

fn compute_reserve_price(
    source: &CurrencyNode,
    reserve_currency_id: &str,
    via: Option<&CurrencyNode>,
    root: Option<&CurrencyNode>,
) -> Option<(f64, Option<f64>, Option<f64>)> {
    if let Some(via_currency) = via {
        let root_currency = root?;
        let via_price_raw = via_currency
            .best_prices
            .get(&root_currency.currency_id)
            .copied()?;
        let dest_price_raw = via_currency.best_prices.get(reserve_currency_id).copied()?;

        let via_price_in_root = normalize_positive(1.0 / via_price_raw)?;
        let dest_price_in_via = normalize_positive(dest_price_raw)?;
        let price = normalize_positive(via_price_in_root * dest_price_in_via)?;
        return Some((price, Some(via_price_in_root), Some(dest_price_in_via)));
    }

    let price_raw = source.best_prices.get(reserve_currency_id).copied()?;
    let price = normalize_positive(price_raw)?;
    Some((price, None, None))
}

fn normalize_positive(value: f64) -> Option<f64> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

fn resolve_export_to_currency_id(
    fractional_converter: &CurrencyNode,
    chain_currency_id: &str,
) -> Option<String> {
    let parent = fractional_converter
        .parent
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let launch = fractional_converter
        .launch_system_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    if parent
        .map(|parent_currency| equals_ignore_case(parent_currency, chain_currency_id))
        .unwrap_or(false)
    {
        launch.map(ToString::to_string)
    } else {
        parent.map(ToString::to_string)
    }
}

fn is_fractional_source(source: &CurrencyNode, chain_currency_id: &str) -> bool {
    is_fractional(source)
        && contains_currency(source, chain_currency_id)
        && (source
            .system_id
            .as_deref()
            .map(|system_id| equals_ignore_case(system_id, chain_currency_id))
            .unwrap_or(false)
            || is_gateway_converter(source))
}

fn is_fractional(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_FRACTIONAL_FLAG)
}

fn is_gateway(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_GATEWAY_FLAG)
}

fn is_gateway_converter(currency: &CurrencyNode) -> bool {
    has_flag(currency.options, IS_GATEWAY_CONVERTER_FLAG)
}

fn has_flag(value: u64, flag: u64) -> bool {
    (value & flag) == flag
}

fn contains_currency(currency: &CurrencyNode, currency_id: &str) -> bool {
    currency
        .currencies
        .iter()
        .any(|reserve_currency| equals_ignore_case(reserve_currency, currency_id))
}

fn contains_ignore(ignore_currencies: &HashSet<String>, currency_id: &str) -> bool {
    ignore_currencies
        .iter()
        .any(|ignored| equals_ignore_case(ignored, currency_id))
}

fn routes_to_quotes(
    routes: &HashMap<String, Vec<RouteCandidate>>,
    currencies: &HashMap<String, CurrencyNode>,
) -> HashMap<String, Vec<BridgeConversionPathQuote>> {
    let mut normalized = HashMap::new();

    for (destination_id, candidates) in routes {
        let Some(destination) = currencies.get(destination_id) else {
            continue;
        };

        let mut quotes = Vec::new();
        for candidate in candidates {
            let export_to = candidate.export_to_id.as_ref().map(|currency_id| {
                currencies
                    .get(currency_id)
                    .map(CurrencyNode::reference)
                    .unwrap_or_else(|| currency_id.clone())
            });
            let via = candidate.via_id.as_ref().map(|currency_id| {
                currencies
                    .get(currency_id)
                    .map(CurrencyNode::reference)
                    .unwrap_or_else(|| currency_id.clone())
            });

            quotes.push(BridgeConversionPathQuote {
                destination_id: destination.currency_id.clone(),
                destination_display_name: destination.display_name(),
                destination_display_ticker: destination.display_ticker(),
                convert_to: Some(destination.reference()),
                export_to,
                via,
                map_to: None,
                price: candidate.price.map(format_decimal),
                via_price_in_root: candidate.via_price_in_root.map(format_decimal),
                dest_price_in_via: candidate.dest_price_in_via.map(format_decimal),
                gateway: candidate.gateway,
                mapping: false,
                bounceback: false,
                eth_destination: false,
            });
        }

        if !quotes.is_empty() {
            normalized.insert(destination_id.clone(), quotes);
        }
    }

    normalized
}

fn merge_currency_nodes_from_list_payload(
    list_payload: &Value,
    currencies: &mut HashMap<String, CurrencyNode>,
) -> Result<(), WalletError> {
    let entries = list_payload
        .as_array()
        .ok_or(WalletError::OperationFailed)?;
    for entry in entries {
        if let Some(currency) = currency_node_from_list_entry(entry) {
            currencies
                .entry(currency.currency_id.clone())
                .or_insert(currency);
        }
    }
    Ok(())
}

fn currency_node_from_list_entry(entry: &Value) -> Option<CurrencyNode> {
    let definition = entry.get("currencydefinition")?;
    currency_node_from_definition(definition, entry.get("bestcurrencystate"))
}

fn currency_node_from_definition(
    definition: &Value,
    best_currency_state: Option<&Value>,
) -> Option<CurrencyNode> {
    let currency_id = definition.get("currencyid").and_then(extract_stringish)?;

    let reserve_currencies = definition
        .get("currencies")
        .and_then(Value::as_array)
        .map(|currencies| {
            currencies
                .iter()
                .filter_map(extract_currency_ref)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(CurrencyNode {
        currency_id,
        fully_qualified_name: definition
            .get("fullyqualifiedname")
            .and_then(extract_stringish),
        name: definition.get("name").and_then(extract_stringish),
        symbol: definition.get("symbol").and_then(extract_stringish),
        options: definition.get("options").and_then(extract_u64).unwrap_or(0),
        system_id: definition.get("systemid").and_then(extract_stringish),
        parent: definition.get("parent").and_then(extract_stringish),
        launch_system_id: definition.get("launchsystemid").and_then(extract_stringish),
        currencies: reserve_currencies,
        best_prices: extract_best_prices(best_currency_state),
    })
}

fn extract_best_prices(best_currency_state: Option<&Value>) -> HashMap<String, f64> {
    let mut prices = HashMap::new();
    let Some(currencies) = best_currency_state
        .and_then(|state| state.get("currencies"))
        .and_then(Value::as_object)
    else {
        return prices;
    };

    for (currency_id, value) in currencies {
        let price = value
            .get("lastconversionprice")
            .and_then(extract_f64)
            .or_else(|| extract_f64(value));
        if let Some(price) = price.and_then(normalize_positive) {
            prices.insert(currency_id.clone(), price);
        }
    }

    prices
}

fn format_decimal(value: f64) -> String {
    let mut output = format!("{value:.16}");
    while output.contains('.') && output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    if output.is_empty() {
        return "0".to_string();
    }
    output
}

fn equals_ignore_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn parse_conversion_paths(
    raw_paths: Value,
) -> Result<HashMap<String, Vec<BridgeConversionPathQuote>>, WalletError> {
    let raw_paths_object = raw_paths.as_object().ok_or(WalletError::OperationFailed)?;
    let mut normalized = HashMap::new();

    for (destination_key, raw_quotes) in raw_paths_object {
        let mut quotes = Vec::new();
        let Some(raw_quotes_array) = raw_quotes.as_array() else {
            normalized.insert(destination_key.clone(), quotes);
            continue;
        };

        for raw_quote in raw_quotes_array {
            let Some(raw_quote_object) = raw_quote.as_object() else {
                continue;
            };

            let destination = raw_quote_object.get("destination");
            let destination_id = extract_destination_id(destination, destination_key);

            let quote = BridgeConversionPathQuote {
                destination_id: destination_id.clone(),
                destination_display_name: extract_destination_display_name(destination),
                destination_display_ticker: extract_destination_display_ticker(destination),
                convert_to: raw_quote_object
                    .get("convertto")
                    .and_then(extract_currency_ref)
                    .or(Some(destination_id)),
                export_to: raw_quote_object
                    .get("exportto")
                    .and_then(extract_currency_ref),
                via: raw_quote_object.get("via").and_then(extract_currency_ref),
                map_to: raw_quote_object
                    .get("mapto")
                    .and_then(extract_currency_ref)
                    .or_else(|| {
                        destination
                            .and_then(|value| value.get("mapto"))
                            .and_then(extract_currency_ref)
                    }),
                price: raw_quote_object.get("price").and_then(extract_stringish),
                via_price_in_root: raw_quote_object
                    .get("viapriceinroot")
                    .and_then(extract_stringish),
                dest_price_in_via: raw_quote_object
                    .get("destpriceinvia")
                    .and_then(extract_stringish),
                gateway: extract_bool(raw_quote_object.get("gateway")),
                mapping: extract_bool(raw_quote_object.get("mapping")),
                bounceback: extract_bool(raw_quote_object.get("bounceback")),
                eth_destination: extract_bool(raw_quote_object.get("ethdest")),
            };

            quotes.push(quote);
        }

        normalized.insert(destination_key.clone(), quotes);
    }

    Ok(normalized)
}

fn extract_stringish(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw.to_string());
    }
    if let Some(raw) = value.as_u64() {
        return Some(raw.to_string());
    }
    value.as_f64().map(|raw| raw.to_string())
}

fn extract_f64(value: &Value) -> Option<f64> {
    if let Some(raw) = value.as_f64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_i64() {
        return Some(raw as f64);
    }
    if let Some(raw) = value.as_u64() {
        return Some(raw as f64);
    }
    value.as_str().and_then(|raw| raw.parse::<f64>().ok())
}

fn extract_u64(value: &Value) -> Option<u64> {
    if let Some(raw) = value.as_u64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_i64() {
        return u64::try_from(raw).ok();
    }
    value.as_str().and_then(|raw| raw.parse::<u64>().ok())
}

fn extract_currency_id(definition: &Value) -> Option<String> {
    definition
        .get("currencyid")
        .and_then(extract_stringish)
        .or_else(|| {
            definition
                .get("fullyqualifiedname")
                .and_then(extract_stringish)
        })
        .or_else(|| definition.get("name").and_then(extract_stringish))
}

fn extract_currency_ref(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        return Some(raw.to_string());
    }

    let object = value.as_object()?;
    object
        .get("fullyqualifiedname")
        .and_then(extract_stringish)
        .or_else(|| object.get("currencyid").and_then(extract_stringish))
        .or_else(|| object.get("address").and_then(extract_stringish))
        .or_else(|| object.get("name").and_then(extract_stringish))
}

fn extract_destination_id(destination: Option<&Value>, fallback: &str) -> String {
    destination
        .and_then(extract_currency_ref)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn extract_destination_display_name(destination: Option<&Value>) -> Option<String> {
    let object = destination?.as_object()?;
    object
        .get("fullyqualifiedname")
        .and_then(extract_stringish)
        .or_else(|| object.get("name").and_then(extract_stringish))
}

fn extract_destination_display_ticker(destination: Option<&Value>) -> Option<String> {
    destination
        .and_then(Value::as_object)
        .and_then(|object| object.get("symbol"))
        .and_then(extract_stringish)
}

fn extract_bool(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{derive_paths_from_list_payloads, parse_conversion_paths};
    use serde_json::json;

    #[test]
    fn parse_conversion_paths_normalizes_common_vrpc_shape() {
        let input = json!({
            "iDest": [
                {
                    "destination": {
                        "currencyid": "iDest",
                        "fullyqualifiedname": "Bridge.DEST",
                        "symbol": "DEST"
                    },
                    "convertto": "Bridge.DEST",
                    "exportto": {
                        "currencyid": "iExport",
                        "fullyqualifiedname": "DEST"
                    },
                    "via": {
                        "currencyid": "iVia",
                        "fullyqualifiedname": "Bridge.vETH"
                    },
                    "price": 1.25,
                    "viapriceinroot": "1.5",
                    "destpriceinvia": 0.83,
                    "gateway": true,
                    "mapping": false,
                    "bounceback": false,
                    "ethdest": false
                }
            ]
        });

        let parsed = parse_conversion_paths(input).expect("parse paths");
        let quotes = parsed.get("iDest").expect("destination key");
        assert_eq!(quotes.len(), 1);

        let quote = &quotes[0];
        assert_eq!(quote.destination_id, "Bridge.DEST");
        assert_eq!(
            quote.destination_display_name.as_deref(),
            Some("Bridge.DEST")
        );
        assert_eq!(quote.destination_display_ticker.as_deref(), Some("DEST"));
        assert_eq!(quote.convert_to.as_deref(), Some("Bridge.DEST"));
        assert_eq!(quote.export_to.as_deref(), Some("DEST"));
        assert_eq!(quote.via.as_deref(), Some("Bridge.vETH"));
        assert_eq!(quote.price.as_deref(), Some("1.25"));
        assert_eq!(quote.via_price_in_root.as_deref(), Some("1.5"));
        assert_eq!(quote.dest_price_in_via.as_deref(), Some("0.83"));
        assert!(quote.gateway);
        assert!(!quote.mapping);
    }

    #[test]
    fn parse_conversion_paths_rejects_non_object_payload() {
        let parsed = parse_conversion_paths(json!([]));
        assert!(parsed.is_err());
    }

    #[test]
    fn fallback_derives_direct_path_quote() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "SRC",
            "systemid": "iChain"
        });
        let list_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iDest",
                    "fullyqualifiedname": "Bridge.DEST",
                    "name": "DEST",
                    "options": 128,
                    "currencies": ["iSource"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        }
                    }
                }
            }
        ]);

        let paths = derive_paths_from_list_payloads(
            &[&list_payload],
            &source_definition,
            None,
            Some("iChain"),
            None,
        )
        .expect("paths");

        let quotes = paths.get("iDest").expect("destination");
        assert!(!quotes.is_empty());
        assert!(quotes.iter().any(|quote| quote.export_to.is_none()));

        let bridge_route = quotes
            .iter()
            .find(|quote| quote.export_to.as_deref() == Some("Bridge.DEST"))
            .expect("bridge route");
        assert_eq!(bridge_route.convert_to.as_deref(), Some("Bridge.DEST"));
        assert_eq!(bridge_route.price.as_deref(), Some("0.5"));
        assert!(bridge_route.gateway);
    }

    #[test]
    fn fallback_derives_via_paths_from_partitioned_payloads() {
        let source_definition = json!({
            "currencyid": "iSource",
            "name": "VRSC",
            "symbol": "VRSC",
            "systemid": "iChain",
            "options": 0
        });

        let local_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iSource",
                    "name": "VRSC",
                    "symbol": "VRSC",
                    "systemid": "iChain",
                    "options": 0
                },
                "bestcurrencystate": {
                    "currencies": {}
                }
            }
        ]);

        let pbaas_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iDest",
                    "fullyqualifiedname": "vUSDC.vETH",
                    "name": "vUSDC",
                    "symbol": "vUSDC",
                    "systemid": "iChain",
                    "options": 0,
                    "currencies": ["iVia"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iVia": {
                            "lastconversionprice": 0.25
                        }
                    }
                }
            }
        ]);

        let imported_payload = json!([
            {
                "currencydefinition": {
                    "currencyid": "iVia",
                    "fullyqualifiedname": "Bridge.vETH",
                    "name": "vETH",
                    "symbol": "vETH",
                    "systemid": "iChain",
                    "options": 1,
                    "currencies": ["iSource", "iDest", "iChain"]
                },
                "bestcurrencystate": {
                    "currencies": {
                        "iSource": {
                            "lastconversionprice": 2.0
                        },
                        "iDest": {
                            "lastconversionprice": 4.0
                        },
                        "iChain": {
                            "lastconversionprice": 1.0
                        }
                    }
                }
            }
        ]);

        let chain_definition = json!({
            "currencyid": "iChain",
            "name": "VRSC",
            "launchsystemid": "iChain",
            "parent": "iParent"
        });

        let paths = derive_paths_from_list_payloads(
            &[&local_payload, &pbaas_payload, &imported_payload],
            &source_definition,
            None,
            Some("iChain"),
            Some(&chain_definition),
        )
        .expect("paths");

        let quotes = paths.get("iDest").expect("destination");
        let via_quote = quotes
            .iter()
            .find(|quote| quote.via.as_deref() == Some("Bridge.vETH"))
            .expect("via route");

        assert_eq!(via_quote.convert_to.as_deref(), Some("vUSDC.vETH"));
        assert_eq!(via_quote.price.as_deref(), Some("2"));
        assert_eq!(via_quote.via_price_in_root.as_deref(), Some("0.5"));
        assert_eq!(via_quote.dest_price_in_via.as_deref(), Some("4"));
    }
}
