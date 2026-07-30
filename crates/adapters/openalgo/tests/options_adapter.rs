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

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{Json, Router, extract::State, routing::post};
use nautilus_openalgo::{
    BookedLegState, OpenAlgoHttpClient, OpenAlgoOptionsAdapter, OptionAction, OptionLegSpec,
    OptionType, PartialSuccessPolicy, RelativeOptionsOrder, RelativeStrike,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Clone, Debug, Default)]
struct MockState {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    option_symbol_calls: Arc<AtomicUsize>,
    basket_calls: Arc<AtomicUsize>,
    partial_submission: bool,
    partial_close: bool,
}

#[tokio::test]
async fn arbitrary_legs_use_canonical_offsets_and_lock_exact_symbols() {
    let (adapter, state) = mock_adapter(false, false).await;
    let order = iron_condor_order(PartialSuccessPolicy::KeepAccepted);

    let first_preview = adapter.preview(&single_leg_order()).await.unwrap();
    let second_preview = adapter.preview(&single_leg_order()).await.unwrap();
    assert_ne!(first_preview[0].symbol, second_preview[0].symbol);

    let mut booking = adapter.submit(&order).await.unwrap();
    assert!(booking.is_complete());
    assert_eq!(booking.legs.len(), 4);
    assert_eq!(booking.legs[0].symbol, "NIFTY30JUL2625300CE");
    assert_eq!(booking.legs[3].symbol, "NIFTY30JUL2624700PE");

    let outcome = adapter.close_open_legs(&mut booking).await.unwrap();
    assert_eq!(outcome.closed.len(), 4);
    assert!(!booking.has_open_legs());

    let requests = state.requests.lock().await;
    assert_request(
        &requests,
        "optionsmultiorder",
        json!({
            "apikey": "KEY",
            "strategy": "RelativeLegs",
            "underlying": "NIFTY",
            "exchange": "NSE_INDEX",
            "legs": [
                {
                    "offset": "OTM3",
                    "option_type": "CE",
                    "action": "BUY",
                    "quantity": 75,
                    "expiry_date": "30JUL26",
                    "splitsize": 0,
                    "pricetype": "MARKET",
                    "product": "MIS",
                    "disclosed_quantity": 0
                },
                {
                    "offset": "OTM1",
                    "option_type": "CE",
                    "action": "SELL",
                    "quantity": 75,
                    "expiry_date": "30JUL26",
                    "splitsize": 0,
                    "pricetype": "MARKET",
                    "product": "MIS",
                    "disclosed_quantity": 0
                },
                {
                    "offset": "OTM1",
                    "option_type": "PE",
                    "action": "SELL",
                    "quantity": 75,
                    "expiry_date": "30JUL26",
                    "splitsize": 0,
                    "pricetype": "MARKET",
                    "product": "MIS",
                    "disclosed_quantity": 0
                },
                {
                    "offset": "OTM3",
                    "option_type": "PE",
                    "action": "BUY",
                    "quantity": 75,
                    "expiry_date": "30JUL26",
                    "splitsize": 0,
                    "pricetype": "MARKET",
                    "product": "MIS",
                    "disclosed_quantity": 0
                }
            ]
        }),
    );
    assert_request(
        &requests,
        "basketorder",
        json!({
            "apikey": "KEY",
            "strategy": "RelativeLegs",
            "orders": [
                {
                    "symbol": "NIFTY30JUL2625300CE",
                    "exchange": "NFO",
                    "action": "SELL",
                    "quantity": 75,
                    "pricetype": "MARKET",
                    "product": "MIS"
                },
                {
                    "symbol": "NIFTY30JUL2625100CE",
                    "exchange": "NFO",
                    "action": "BUY",
                    "quantity": 75,
                    "pricetype": "MARKET",
                    "product": "MIS"
                },
                {
                    "symbol": "NIFTY30JUL2624900PE",
                    "exchange": "NFO",
                    "action": "BUY",
                    "quantity": 75,
                    "pricetype": "MARKET",
                    "product": "MIS"
                },
                {
                    "symbol": "NIFTY30JUL2624700PE",
                    "exchange": "NFO",
                    "action": "SELL",
                    "quantity": 75,
                    "pricetype": "MARKET",
                    "product": "MIS"
                }
            ]
        }),
    );
}

#[tokio::test]
async fn partial_submission_can_close_only_the_accepted_exact_leg() {
    let (adapter, state) = mock_adapter(true, false).await;
    let order = RelativeOptionsOrder {
        partial_success: PartialSuccessPolicy::CloseAccepted,
        ..single_leg_pair_order()
    };

    let booking = adapter.submit(&order).await.unwrap();
    assert!(!booking.is_complete());
    assert_eq!(booking.rejected[0].id, "put");
    assert!(matches!(
        booking.legs[0].state,
        BookedLegState::Closed { .. }
    ));
    assert!(booking.recovery_error.is_none());

    let requests = state.requests.lock().await;
    let basket = requests
        .iter()
        .find(|(endpoint, _)| endpoint == "basketorder")
        .map(|(_, payload)| payload)
        .unwrap();
    assert_eq!(basket["orders"].as_array().unwrap().len(), 1);
    assert_eq!(basket["orders"][0]["symbol"], "NIFTY30JUL2625000CE");
}

#[tokio::test]
async fn close_retry_excludes_already_closed_exact_legs() {
    let (adapter, state) = mock_adapter(false, true).await;
    let mut booking = adapter
        .submit(&iron_condor_order(PartialSuccessPolicy::KeepAccepted))
        .await
        .unwrap();

    let first = adapter.close_open_legs(&mut booking).await.unwrap();
    assert_eq!(first.closed.len(), 3);
    assert_eq!(first.rejected, vec!["short-call"]);
    assert!(booking.has_open_legs());

    let second = adapter.close_open_legs(&mut booking).await.unwrap();
    assert_eq!(second.closed, vec!["short-call"]);
    assert!(!booking.has_open_legs());

    let requests = state.requests.lock().await;
    let baskets = requests
        .iter()
        .filter(|(endpoint, _)| endpoint == "basketorder")
        .map(|(_, payload)| payload)
        .collect::<Vec<_>>();
    assert_eq!(baskets.len(), 2);
    assert_eq!(baskets[0]["orders"].as_array().unwrap().len(), 4);
    assert_eq!(baskets[1]["orders"].as_array().unwrap().len(), 1);
    assert_eq!(baskets[1]["orders"][0]["symbol"], "NIFTY30JUL2625100CE");
}

fn iron_condor_order(partial_success: PartialSuccessPolicy) -> RelativeOptionsOrder {
    RelativeOptionsOrder {
        strategy: "RelativeLegs".to_string(),
        underlying: "NIFTY".to_string(),
        exchange: "NSE_INDEX".to_string(),
        options_exchange: "NFO".to_string(),
        legs: vec![
            leg(
                "long-call",
                OptionType::Ce,
                RelativeStrike::Otm(3),
                OptionAction::Buy,
            ),
            leg(
                "short-call",
                OptionType::Ce,
                RelativeStrike::Otm(1),
                OptionAction::Sell,
            ),
            leg(
                "short-put",
                OptionType::Pe,
                RelativeStrike::Otm(1),
                OptionAction::Sell,
            ),
            leg(
                "long-put",
                OptionType::Pe,
                RelativeStrike::Otm(3),
                OptionAction::Buy,
            ),
        ],
        partial_success,
    }
}

fn single_leg_order() -> RelativeOptionsOrder {
    RelativeOptionsOrder {
        strategy: "Preview".to_string(),
        underlying: "NIFTY".to_string(),
        exchange: "NSE_INDEX".to_string(),
        options_exchange: "NFO".to_string(),
        legs: vec![leg(
            "moving",
            OptionType::Ce,
            RelativeStrike::Itm(1),
            OptionAction::Buy,
        )],
        partial_success: PartialSuccessPolicy::KeepAccepted,
    }
}

fn single_leg_pair_order() -> RelativeOptionsOrder {
    RelativeOptionsOrder {
        strategy: "RelativeLegs".to_string(),
        underlying: "NIFTY".to_string(),
        exchange: "NSE_INDEX".to_string(),
        options_exchange: "NFO".to_string(),
        legs: vec![
            leg(
                "call",
                OptionType::Ce,
                RelativeStrike::Atm,
                OptionAction::Buy,
            ),
            leg(
                "put",
                OptionType::Pe,
                RelativeStrike::Atm,
                OptionAction::Buy,
            ),
        ],
        partial_success: PartialSuccessPolicy::KeepAccepted,
    }
}

fn leg(
    id: &str,
    option_type: OptionType,
    strike: RelativeStrike,
    action: OptionAction,
) -> OptionLegSpec {
    OptionLegSpec::market(id, option_type, strike, action, 75, "30-Jul-26")
}

async fn mock_adapter(
    partial_submission: bool,
    partial_close: bool,
) -> (OpenAlgoOptionsAdapter, MockState) {
    let state = MockState {
        partial_submission,
        partial_close,
        ..MockState::default()
    };
    let app = Router::new()
        .route("/api/v1/optionsymbol", post(option_symbol))
        .route("/api/v1/optionsmultiorder", post(options_multi_order))
        .route("/api/v1/basketorder", post(basket_order))
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = OpenAlgoHttpClient::new("KEY", &host, "v1", "ws://127.0.0.1");
    (OpenAlgoOptionsAdapter::new(client), state)
}

async fn option_symbol(State(state): State<MockState>, Json(payload): Json<Value>) -> Json<Value> {
    capture(&state, "optionsymbol", payload).await;
    let call = state.option_symbol_calls.fetch_add(1, Ordering::SeqCst);
    let strike = if call == 0 { 24_900 } else { 25_000 };
    Json(json!({
        "status": "success",
        "symbol": format!("NIFTY30JUL26{strike}CE"),
        "exchange": "NFO",
        "lotsize": 75,
        "tick_size": 0.05,
        "freeze_qty": 1800,
        "underlying_ltp": if call == 0 { 25010.0 } else { 25110.0 }
    }))
}

async fn options_multi_order(
    State(state): State<MockState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    capture(&state, "optionsmultiorder", payload).await;
    if state.partial_submission {
        return Json(json!({
            "status": "partial",
            "underlying": "NIFTY",
            "underlying_ltp": 25010.0,
            "mode": "analyze",
            "results": [
                {
                    "leg": 1,
                    "symbol": "NIFTY30JUL2625000CE",
                    "exchange": "NFO",
                    "offset": "ATM",
                    "option_type": "CE",
                    "action": "BUY",
                    "status": "success",
                    "orderid": "SB-CALL"
                },
                {
                    "leg": 2,
                    "offset": "ATM",
                    "option_type": "PE",
                    "action": "BUY",
                    "status": "error",
                    "message": "insufficient funds"
                }
            ]
        }));
    }

    Json(json!({
        "status": "success",
        "underlying": "NIFTY",
        "underlying_ltp": 25010.0,
        "mode": "analyze",
        "results": [
            result(1, "NIFTY30JUL2625300CE", "OTM3", "CE", "BUY"),
            result(2, "NIFTY30JUL2625100CE", "OTM1", "CE", "SELL"),
            result(3, "NIFTY30JUL2624900PE", "OTM1", "PE", "SELL"),
            result(4, "NIFTY30JUL2624700PE", "OTM3", "PE", "BUY")
        ]
    }))
}

fn result(leg: i32, symbol: &str, offset: &str, option_type: &str, action: &str) -> Value {
    json!({
        "leg": leg,
        "symbol": symbol,
        "exchange": "NFO",
        "offset": offset,
        "option_type": option_type,
        "action": action,
        "status": "success",
        "orderid": format!("SB-{leg}")
    })
}

async fn basket_order(State(state): State<MockState>, Json(payload): Json<Value>) -> Json<Value> {
    capture(&state, "basketorder", payload.clone()).await;
    let call = state.basket_calls.fetch_add(1, Ordering::SeqCst);
    let results = payload["orders"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, order)| {
            let rejected = state.partial_close && call == 0 && index == 1;
            json!({
                "symbol": order["symbol"],
                "status": if rejected { "error" } else { "success" },
                "orderid": if rejected {
                    Value::Null
                } else {
                    json!(format!("SB-CLOSE-{}", index + 1))
                }
            })
        })
        .collect::<Vec<_>>();
    Json(json!({"status": "success", "results": results}))
}

async fn capture(state: &MockState, endpoint: &str, payload: Value) {
    state
        .requests
        .lock()
        .await
        .push((endpoint.to_string(), payload));
}

fn assert_request(requests: &[(String, Value)], endpoint: &str, expected: Value) {
    let actual = requests
        .iter()
        .find(|(name, _)| name == endpoint)
        .map_or_else(
            || panic!("missing {endpoint} request"),
            |(_, payload)| payload,
        );
    assert_eq!(actual, &expected);
}
