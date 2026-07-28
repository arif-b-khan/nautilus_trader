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

use anyhow::Result;
use openalgo::OpenAlgo;
use serde::Serialize;
use std::{fmt, sync::Arc};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "nautilus_trader._libnautilus.openalgo",
        from_py_object
    )
)]
#[derive(Clone)]
pub struct OpenAlgoHttpClient {
    inner: Arc<OpenAlgo>,
}

impl OpenAlgoHttpClient {
    #[must_use]
    pub fn new(api_key: &str, host: &str, version: &str, ws_url: &str) -> Self {
        Self {
            inner: Arc::new(OpenAlgo::with_config(api_key, host, version, ws_url)),
        }
    }

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
                    )
                    .await?
            }
            _ => {
                self.inner
                    .place_order(
                        strategy, symbol, action, exchange, pricetype, product, quantity,
                    )
                    .await?
            }
        };

        to_json(response)
    }

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
            )
            .await?;
        to_json(response)
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
