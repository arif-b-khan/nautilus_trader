// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use std::{collections::HashMap, fmt, sync::Arc};

use anyhow::Result;
use openalgo::{
    AnalyzerStatusResponse, BasketOrderItem, BasketOrderResponse, ExpiryResponse, OpenAlgo,
    OpenAlgoClient, OptionChainResponse, OptionGreeksResponse, OptionSymbolResponse, OptionsLeg,
    OptionsMultiOrderResponse, OptionsOrderResponse, OrderResponse, QuotesResponse,
    SyntheticFutureResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::models::{
    MultiOptionGreeksInstrument, MultiOptionGreeksRequest, MultiOptionGreeksResponse,
};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader._libnautilus.openalgo", skip_from_py_object)
)]
#[derive(Clone)]
pub struct OpenAlgoHttpClient {
    inner: Arc<OpenAlgo>,
    raw: Arc<OpenAlgoClient>,
}

impl OpenAlgoHttpClient {
    #[must_use]
    pub fn new(api_key: &str, host: &str, version: &str, ws_url: &str) -> Self {
        Self {
            inner: Arc::new(OpenAlgo::with_config(api_key, host, version, ws_url)),
            raw: Arc::new(OpenAlgoClient::new(api_key, host, version, ws_url)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn place_order(
        &self,
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
        price: Option<&str>,
        trigger_price: Option<&str>,
    ) -> Result<String> {
        let pricetype_upper = pricetype.to_ascii_uppercase();
        let response = match pricetype_upper.as_str() {
            "LIMIT" => {
                self.inner
                    .place_limit_order(
                        strategy,
                        symbol,
                        action,
                        exchange,
                        product,
                        quantity,
                        price.unwrap_or("0"),
                        None,
                    )
                    .await?
            }
            "SL" => {
                self.inner
                    .place_sl_order(
                        strategy,
                        symbol,
                        action,
                        exchange,
                        product,
                        quantity,
                        price.unwrap_or("0"),
                        trigger_price.unwrap_or("0"),
                        None,
                    )
                    .await?
            }
            _ => {
                self.inner
                    .place_order(
                        strategy, symbol, action, exchange, pricetype, product, quantity, None,
                    )
                    .await?
            }
        };

        to_json(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn modify_order(
        &self,
        orderid: &str,
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
        price: &str,
    ) -> Result<String> {
        let response = self
            .inner
            .modify_order(
                orderid, strategy, symbol, action, exchange, pricetype, product, quantity, price,
                None, None, None,
            )
            .await?;
        to_json(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn place_order_typed(
        &self,
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
    ) -> Result<OrderResponse> {
        Ok(self
            .inner
            .place_order(
                strategy, symbol, action, exchange, pricetype, product, quantity, None,
            )
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn options_order(
        &self,
        strategy: &str,
        underlying: &str,
        exchange: &str,
        offset: &str,
        option_type: &str,
        action: &str,
        quantity: &str,
        pricetype: &str,
        product: &str,
        expiry_date: Option<&str>,
        strike_int: Option<i32>,
        extra: Option<HashMap<String, Value>>,
    ) -> Result<OptionsOrderResponse> {
        Ok(self
            .inner
            .options_order(
                strategy,
                underlying,
                exchange,
                offset,
                option_type,
                action,
                quantity,
                pricetype,
                product,
                expiry_date,
                strike_int,
                extra,
            )
            .await?)
    }

    pub async fn options_multi_order(
        &self,
        strategy: &str,
        underlying: &str,
        exchange: &str,
        expiry_date: &str,
        legs: Vec<OptionsLeg>,
    ) -> Result<OptionsMultiOrderResponse> {
        Ok(self
            .inner
            .options_multi_order(strategy, underlying, exchange, expiry_date, legs)
            .await?)
    }

    pub async fn basket_order(
        &self,
        strategy: &str,
        orders: Vec<BasketOrderItem>,
    ) -> Result<BasketOrderResponse> {
        Ok(self.inner.basket_order(strategy, orders).await?)
    }

    pub async fn option_chain(
        &self,
        underlying: &str,
        exchange: &str,
        expiry_date: &str,
        strike_count: Option<i32>,
    ) -> Result<OptionChainResponse> {
        let response = match strike_count {
            Some(count) => {
                self.inner
                    .data
                    .option_chain_strikes(underlying, exchange, expiry_date, count)
                    .await?
            }
            None => {
                self.inner
                    .option_chain(underlying, exchange, expiry_date)
                    .await?
            }
        };
        Ok(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn option_symbol(
        &self,
        underlying: &str,
        exchange: &str,
        offset: &str,
        option_type: &str,
        expiry_date: Option<&str>,
        strategy: Option<&str>,
        strike_int: Option<i32>,
        extra: Option<HashMap<String, Value>>,
    ) -> Result<OptionSymbolResponse> {
        Ok(self
            .inner
            .option_symbol(
                underlying,
                exchange,
                offset,
                option_type,
                expiry_date,
                strategy,
                strike_int,
                extra,
            )
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn option_greeks(
        &self,
        symbol: &str,
        exchange: &str,
        interest_rate: Option<f64>,
        forward_price: Option<f64>,
        underlying_symbol: Option<&str>,
        underlying_exchange: Option<&str>,
        expiry_time: Option<&str>,
        extra: Option<HashMap<String, Value>>,
    ) -> Result<OptionGreeksResponse> {
        Ok(self
            .inner
            .option_greeks(
                symbol,
                exchange,
                interest_rate,
                forward_price,
                underlying_symbol,
                underlying_exchange,
                expiry_time,
                extra,
            )
            .await?)
    }

    pub async fn multi_option_greeks(
        &self,
        symbols: Vec<MultiOptionGreeksInstrument>,
        interest_rate: Option<f64>,
        expiry_time: Option<&str>,
    ) -> Result<MultiOptionGreeksResponse> {
        let request = MultiOptionGreeksRequest {
            apikey: self.raw.api_key.clone(),
            symbols,
            interest_rate,
            expiry_time: expiry_time.map(str::to_string),
        };
        Ok(self.raw.post("multioptiongreeks", &request).await?)
    }

    pub async fn expiry(
        &self,
        symbol: &str,
        exchange: &str,
        instrument_type: &str,
    ) -> Result<ExpiryResponse> {
        Ok(self.inner.expiry(symbol, exchange, instrument_type).await?)
    }

    pub async fn synthetic_future(
        &self,
        underlying: &str,
        exchange: &str,
        expiry_date: &str,
    ) -> Result<SyntheticFutureResponse> {
        Ok(self
            .inner
            .synthetic_future(underlying, exchange, expiry_date)
            .await?)
    }

    pub async fn quotes(&self, symbol: &str, exchange: &str) -> Result<QuotesResponse> {
        Ok(self.inner.quotes(symbol, exchange).await?)
    }

    pub async fn history_range(
        &self,
        symbol: &str,
        exchange: &str,
        interval: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Value> {
        Ok(self
            .inner
            .history_range(symbol, exchange, interval, start_date, end_date)
            .await?)
    }

    pub async fn analyzer_status(&self) -> Result<AnalyzerStatusResponse> {
        Ok(self.inner.analyzer_status().await?)
    }

    pub(crate) fn api_key(&self) -> String {
        self.raw.api_key.clone()
    }

    pub(crate) async fn post<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        Ok(self.raw.post(endpoint, request).await?)
    }

    pub async fn cancel_order(&self, orderid: &str, strategy: &str) -> Result<String> {
        let response = self.inner.cancel_order(orderid, strategy).await?;
        to_json(response)
    }

    pub async fn cancel_all_order(&self, strategy: &str) -> Result<String> {
        let response = self.inner.cancel_all_order(strategy).await?;
        to_json(response)
    }

    pub async fn order_status(&self, orderid: &str, strategy: &str) -> Result<String> {
        let response = self.inner.order_status(orderid, strategy).await?;
        to_json(response)
    }

    pub async fn funds(&self) -> Result<String> {
        let response = self.inner.funds().await?;
        to_json(response)
    }

    pub async fn orderbook(&self) -> Result<String> {
        let response = self.inner.orderbook().await?;
        to_json(response)
    }

    pub async fn tradebook(&self) -> Result<String> {
        let response = self.inner.tradebook().await?;
        to_json(response)
    }

    pub async fn positionbook(&self) -> Result<String> {
        let response = self.inner.positionbook().await?;
        to_json(response)
    }
}

fn to_json<T: Serialize>(value: T) -> Result<String> {
    Ok(serde_json::to_string(&value)?)
}

impl fmt::Debug for OpenAlgoHttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAlgoHttpClient").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Json, Router, extract::State, routing::post};
    use serde_json::{Value, json};
    use tokio::{net::TcpListener, sync::Mutex};

    use super::*;

    type RequestCapture = Arc<Mutex<Option<Value>>>;

    #[tokio::test]
    async fn place_limit_order_uses_openalgo_sdk_endpoint_and_payload() {
        let captured: RequestCapture = Arc::new(Mutex::new(None));
        let app = Router::new()
            .route("/api/v1/placeorder", post(capture_place_order))
            .with_state(captured.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = OpenAlgoHttpClient::new("KEY", &host, "v1", "ws://127.0.0.1:8765");
        let raw = client
            .place_order(
                "NautilusTrader",
                "RELIANCE",
                "BUY",
                "NSE",
                "LIMIT",
                "MIS",
                "1",
                Some("2500.00"),
                None,
            )
            .await
            .unwrap();

        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            response,
            json!({"status": "success", "orderid": "250408000989443", "message": null})
        );

        let request = captured.lock().await.take().unwrap();
        assert_eq!(
            request,
            json!({
                "apikey": "KEY",
                "strategy": "NautilusTrader",
                "symbol": "RELIANCE",
                "action": "BUY",
                "exchange": "NSE",
                "pricetype": "LIMIT",
                "product": "MIS",
                "quantity": "1",
                "price": "2500.00"
            })
        );
    }

    async fn capture_place_order(
        State(captured): State<RequestCapture>,
        Json(payload): Json<Value>,
    ) -> Json<Value> {
        *captured.lock().await = Some(payload);
        Json(json!({"status": "success", "orderid": "250408000989443"}))
    }
}
