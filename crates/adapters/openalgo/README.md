# Nautilus OpenAlgo Adapter

Rust-native OpenAlgo integration for NautilusTrader, built on the official
`openalgo` crate.

## Relative Options API

`OpenAlgoHttpClient` exposes typed Rust methods for:

- single and multi-leg options orders
- basket orders using exact contract symbols
- option chain and expiry discovery
- option symbol lookup
- single and multi-option Greeks
- synthetic futures
- option quotes and historical data
- analyzer status

`OpenAlgoOptionsAdapter` accepts an underlying and one to twenty arbitrary
relative option legs. It is not tied to a named strategy or index.

Relative strikes accept both the user-facing bracket notation and OpenAlgo's
canonical notation:

- `[atm-0]`, `ATM`, and `ATM0` become `ATM`
- `[itm-1]`, `ITM-1`, and `ITM1` become `ITM1`
- `[otm-1]`, `OTM-1`, and `OTM1` become `OTM1`

Calling `preview` resolves the current exact symbols without locking them. A
later preview may return different symbols as the underlying moves. Calling
`submit` sends the relative selectors and stores the exact symbols returned by
OpenAlgo. `close_open_legs` always uses those booked symbols and never
recalculates ATM, ITM, or OTM.

Multi-leg responses are not atomic. `PartialSuccessPolicy::KeepAccepted`
retains accepted exact contracts for the caller, while
`PartialSuccessPolicy::CloseAccepted` immediately attempts to close only the
accepted symbols. Failed closes remain retryable.

## Analyzer Example

The example reads arbitrary legs from JSON. It previews them first, then refuses
to submit unless the server confirms analyzer mode and an explicit environment
confirmation is present.

```bash
export OPENALGO_API_KEY="..."
export OPENALGO_HOST="https://your-openalgo-host"
export OPENALGO_UNDERLYING="NIFTY"
export OPENALGO_UNDERLYING_EXCHANGE="NSE_INDEX"
export OPENALGO_OPTIONS_EXCHANGE="NFO"
export OPENALGO_STRATEGY="RelativeLegs"
export OPENALGO_OPTION_LEGS='[
  {"id":"call-hedge","option_type":"CE","strike":"[otm-3]","action":"BUY","quantity":75,"expiry_date":"30JUL26"},
  {"id":"call-short","option_type":"CE","strike":"[otm-1]","action":"SELL","quantity":75,"expiry_date":"30JUL26"},
  {"id":"put-short","option_type":"PE","strike":"[otm-1]","action":"SELL","quantity":75,"expiry_date":"30JUL26"},
  {"id":"put-hedge","option_type":"PE","strike":"[otm-3]","action":"BUY","quantity":75,"expiry_date":"30JUL26"}
]'
export OPENALGO_CONFIRM_ANALYZER_ORDER="yes"

cargo run -p nautilus-openalgo --example relative_options_analyzer
```

The example submits the analyzer order and prints each exact booked contract.
It does not close the contracts automatically.
