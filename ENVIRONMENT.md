# Verus Express - Environment Variable Reference

This document describes how to run `Verus Express` using environment variables and explains what each variable controls.

## Example

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 \
VRPC_TESTNET_URL=http://192.168.223.38:27001/ \
VRPC_TIMEOUT=300 \
verus-express-binary
```

---


### `WEBKIT_DISABLE_COMPOSITING_MODE`

**Type:** Boolean (`0` or `1`)  
**Example:** `WEBKIT_DISABLE_COMPOSITING_MODE=1`

Disables WebKit compositing mode.

**When to use this:**
- Running in headless environments or virtual machines
- Experiencing GPU or rendering-related crashes
- Using older or unsupported graphics drivers

Set to `1` to disable compositing.  
Leave unset or set to `0` to use the default behavior.


---


### `VRPC_TESTNET_URL`

**Type:** URL  
**Example:** `VRPC_TESTNET_URL=http://192.168.223.38:27001/`

Specifies the Verus RPC endpoint used by `Verus Express`.

**Notes:**
- Must include the protocol (`http://` or `https://`)
- Should point to an active and reachable RPC service
- Commonly used for local, development, or testnet environments

If this variable is not set, `Verus Express` may fall back to a default endpoint or fail to start, depending on configuration.


---


### `VRPC_TIMEOUT`

**Type:** Integer (seconds)  
**Example:** `VRPC_TIMEOUT=300`

Sets the maximum amount of time `Verus Express` will wait for an RPC response before timing out.

**Important:** If the RPC server is served by the Rust RPC, it must be configured to support timeouts at least as long as this value. Otherwise, requests may fail even if `VRPC_TIMEOUT` is set higher.

**Recommendations:**
- Increase for slow networks or long-running RPC calls
- Decrease for CI pipelines or fail-fast workflows

A value of `300` equals a 5-minute timeout.


