// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::post};
use nautilus_openalgo::OpenAlgoHttpClient;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::Mutex};

type RequestCapture = Arc<Mutex<Option<Value>>>;

#[derive(Debug)]
struct SimpleOpenAlgoStrategy {
    client: OpenAlgoHttpClient,
    strategy: &'static str,
    symbol: &'static str,
    exchange: &'static str,
    product: &'static str,
}

impl SimpleOpenAlgoStrategy {
    async fn buy_once(&self) -> anyhow::Result<Value> {
        let raw = self
            .client
            .place_order(
                self.strategy,
                self.symbol,
                "BUY",
                self.exchange,
                "MARKET",
                self.product,
                "1",
                None,
                None,
            )
            .await?;

        Ok(serde_json::from_str(&raw)?)
    }
}

#[tokio::test]
async fn simple_strategy_places_market_order_through_openalgo_adapter() {
    let captured: RequestCapture = Arc::new(Mutex::new(None));
    let app = Router::new()
        .route("/api/v1/placeorder", post(capture_place_order))
        .with_state(captured.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let strategy = SimpleOpenAlgoStrategy {
        client: OpenAlgoHttpClient::new("KEY", &host, "v1", "ws://127.0.0.1:8765"),
        strategy: "NautilusTrader",
        symbol: "RELIANCE",
        exchange: "NSE",
        product: "MIS",
    };

    let response = strategy.buy_once().await.unwrap();
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
            "pricetype": "MARKET",
            "product": "MIS",
            "quantity": "1"
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
