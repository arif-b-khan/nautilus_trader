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

use std::collections::HashSet;

use anyhow::{Context, Result, bail, ensure};
use openalgo::BasketOrderItem;

use super::models::{
    BookedLegState, BookedOptionLeg, OptionsBooking, OptionsCloseOutcome, OptionsMultiOrderRequest,
    OptionsMultiOrderResponse, PartialSuccessPolicy, RejectedOptionLeg, RelativeOptionsOrder,
    ResolvedOptionLeg, normalize_expiry,
};
use crate::OpenAlgoHttpClient;

/// Generic adapter-level relative-options selection and execution.
#[derive(Clone, Debug)]
pub struct OpenAlgoOptionsAdapter {
    client: OpenAlgoHttpClient,
}

impl OpenAlgoOptionsAdapter {
    #[must_use]
    pub const fn new(client: OpenAlgoHttpClient) -> Self {
        Self { client }
    }

    /// Resolves the latest exact symbol for every relative leg without placing orders.
    ///
    /// Calling this method again may return different symbols when the underlying ATM
    /// strike moves. A preview does not lock a contract.
    pub async fn preview(&self, order: &RelativeOptionsOrder) -> Result<Vec<ResolvedOptionLeg>> {
        order.validate()?;
        let mut resolved = Vec::with_capacity(order.legs.len());

        for leg in &order.legs {
            let expiry = normalize_expiry(&leg.expiry_date);
            let response = self
                .client
                .option_symbol(
                    &order.underlying,
                    &order.exchange,
                    &leg.strike.to_string(),
                    &leg.option_type.to_string(),
                    Some(&expiry),
                    Some(&order.strategy),
                    None,
                    None,
                )
                .await
                .with_context(|| format!("resolving option leg {}", leg.id))?;
            ensure!(
                is_success(&response.status),
                "leg {} resolution failed: {}",
                leg.id,
                response
                    .message
                    .as_deref()
                    .unwrap_or("OpenAlgo returned an error")
            );
            let symbol = response
                .symbol
                .context(format!("leg {} resolution omitted symbol", leg.id))?;
            let exchange = response
                .exchange
                .unwrap_or_else(|| order.options_exchange.clone());
            let lot_size = response
                .lotsize
                .context(format!("leg {} resolution omitted lot size", leg.id))?;
            ensure!(
                leg.quantity % lot_size == 0,
                "leg {} quantity {} is not a multiple of lot size {}",
                leg.id,
                leg.quantity,
                lot_size
            );

            resolved.push(ResolvedOptionLeg {
                id: leg.id.clone(),
                requested_strike: leg.strike,
                option_type: leg.option_type,
                expiry_date: expiry,
                symbol,
                exchange,
                lot_size,
                tick_size: response.tick_size,
                freeze_quantity: response.freeze_qty,
                underlying_ltp: response.underlying_ltp,
            });
        }
        Ok(resolved)
    }

    /// Submits one to twenty arbitrary relative option legs.
    ///
    /// Exact symbols returned for accepted legs are retained in the booking. Relative
    /// selectors are never used to close a booked leg.
    pub async fn submit(&self, order: &RelativeOptionsOrder) -> Result<OptionsBooking> {
        order.validate()?;
        let request = OptionsMultiOrderRequest {
            apikey: self.client.api_key(),
            strategy: order.strategy.clone(),
            underlying: order.underlying.clone(),
            exchange: order.exchange.clone(),
            legs: order.legs.iter().map(Into::into).collect(),
        };
        let response: OptionsMultiOrderResponse = self
            .client
            .post("optionsmultiorder", &request)
            .await
            .context("submitting relative option legs")?;
        let mut booking = self.parse_booking(order, response)?;

        if !booking.rejected.is_empty()
            && !booking.legs.is_empty()
            && order.partial_success == PartialSuccessPolicy::CloseAccepted
        {
            match self.close_open_legs(&mut booking).await {
                Ok(outcome) if !outcome.rejected.is_empty() => {
                    booking.recovery_error = Some(format!(
                        "automatic close rejected legs: {}",
                        outcome.rejected.join(", ")
                    ));
                }
                Err(error) => booking.recovery_error = Some(error.to_string()),
                _ => {}
            }
        }
        Ok(booking)
    }

    /// Closes every still-open booked leg using its exact resolved symbol.
    ///
    /// Successfully closed legs are not included in later retries.
    pub async fn close_open_legs(
        &self,
        booking: &mut OptionsBooking,
    ) -> Result<OptionsCloseOutcome> {
        let open = booking
            .legs
            .iter()
            .enumerate()
            .filter(|(_, leg)| !matches!(leg.state, BookedLegState::Closed { .. }))
            .map(|(index, leg)| {
                (
                    index,
                    BasketOrderItem::new(
                        &leg.symbol,
                        &leg.exchange,
                        &leg.opening_action.opposite().to_string(),
                        leg.quantity,
                        "MARKET",
                        &leg.product.to_string(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        if open.is_empty() {
            return Ok(OptionsCloseOutcome::default());
        }

        let response = self
            .client
            .basket_order(
                &booking.strategy,
                open.iter().map(|(_, order)| order.clone()).collect(),
            )
            .await
            .context("closing exact booked option legs")?;
        let results = response.results.unwrap_or_default();
        let mut outcome = OptionsCloseOutcome::default();

        for (index, _) in open {
            let leg = &mut booking.legs[index];
            match results.iter().find(|result| result.symbol == leg.symbol) {
                Some(result) if is_success(&result.status) => {
                    let order_id = result
                        .orderid
                        .clone()
                        .context("successful close response omitted order ID")?;
                    leg.state = BookedLegState::Closed { order_id };
                    outcome.closed.push(leg.id.clone());
                }
                _ => {
                    leg.state = BookedLegState::CloseRejected;
                    outcome.rejected.push(leg.id.clone());
                }
            }
        }
        Ok(outcome)
    }

    fn parse_booking(
        &self,
        order: &RelativeOptionsOrder,
        response: OptionsMultiOrderResponse,
    ) -> Result<OptionsBooking> {
        let results = response.results.unwrap_or_default();
        let mut seen = HashSet::new();
        let mut booked = Vec::new();
        let mut rejected = Vec::new();

        for result in results {
            ensure!(
                result.leg > 0 && (result.leg as usize) <= order.legs.len(),
                "OpenAlgo returned invalid leg number {}",
                result.leg
            );
            ensure!(
                seen.insert(result.leg),
                "OpenAlgo returned duplicate leg number {}",
                result.leg
            );
            let spec = &order.legs[result.leg as usize - 1];
            validate_echoed_leg(spec, &result)?;

            if is_success(&result.status) {
                booked.push(BookedOptionLeg {
                    id: spec.id.clone(),
                    requested_strike: spec.strike,
                    option_type: spec.option_type,
                    opening_action: spec.action,
                    quantity: spec.quantity,
                    product: spec.product,
                    expiry_date: normalize_expiry(&spec.expiry_date),
                    symbol: result
                        .symbol
                        .context(format!("accepted leg {} omitted symbol", spec.id))?,
                    exchange: result
                        .exchange
                        .unwrap_or_else(|| order.options_exchange.clone()),
                    opening_order_id: result
                        .orderid
                        .context(format!("accepted leg {} omitted order ID", spec.id))?,
                    state: BookedLegState::Open,
                });
            } else {
                rejected.push(RejectedOptionLeg {
                    id: spec.id.clone(),
                    status: result.status,
                    message: result.message,
                });
            }
        }

        for (index, spec) in order.legs.iter().enumerate() {
            if !seen.contains(&((index + 1) as i32)) {
                rejected.push(RejectedOptionLeg {
                    id: spec.id.clone(),
                    status: response.status.clone(),
                    message: response.message.clone(),
                });
            }
        }
        if booked.is_empty() && rejected.is_empty() {
            bail!(
                "OpenAlgo returned no leg results: {}",
                response.message.as_deref().unwrap_or("unknown error")
            );
        }

        Ok(OptionsBooking {
            strategy: order.strategy.clone(),
            underlying: response
                .underlying
                .unwrap_or_else(|| order.underlying.clone()),
            underlying_ltp: response.underlying_ltp,
            mode: response.mode,
            legs: booked,
            rejected,
            recovery_error: None,
        })
    }
}

fn validate_echoed_leg(
    spec: &super::models::OptionLegSpec,
    result: &super::models::OptionsMultiOrderResult,
) -> Result<()> {
    if let Some(offset) = result.offset.as_deref() {
        ensure!(
            offset.eq_ignore_ascii_case(&spec.strike.to_string()),
            "OpenAlgo returned unexpected offset {offset} for leg {}",
            spec.id
        );
    }
    if let Some(option_type) = result.option_type.as_deref() {
        ensure!(
            option_type.eq_ignore_ascii_case(&spec.option_type.to_string()),
            "OpenAlgo returned unexpected option type {option_type} for leg {}",
            spec.id
        );
    }
    if let Some(action) = result.action.as_deref() {
        ensure!(
            action.eq_ignore_ascii_case(&spec.action.to_string()),
            "OpenAlgo returned unexpected action {action} for leg {}",
            spec.id
        );
    }
    Ok(())
}

fn is_success(status: &str) -> bool {
    status.eq_ignore_ascii_case("success")
}
