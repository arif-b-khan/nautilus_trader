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

use std::{fmt, str::FromStr};

use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

/// A strike selected relative to the moving at-the-money strike.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RelativeStrike {
    Atm,
    Itm(u8),
    Otm(u8),
}

impl RelativeStrike {
    fn validate_level(kind: &str, level: u8) -> Result<u8> {
        ensure!(
            (1..=50).contains(&level),
            "{kind} level must be between 1 and 50"
        );
        Ok(level)
    }
}

impl fmt::Display for RelativeStrike {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Atm => f.write_str("ATM"),
            Self::Itm(level) => write!(f, "ITM{level}"),
            Self::Otm(level) => write!(f, "OTM{level}"),
        }
    }
}

impl FromStr for RelativeStrike {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_uppercase();
        if normalized == "ATM" || normalized == "ATM-0" || normalized == "ATM0" {
            return Ok(Self::Atm);
        }

        let parse_level = |prefix: &str| -> Result<u8> {
            let raw = normalized
                .strip_prefix(prefix)
                .ok_or_else(|| anyhow::anyhow!("invalid relative strike: {value}"))?
                .strip_prefix('-')
                .unwrap_or_else(|| normalized.strip_prefix(prefix).unwrap_or_default());
            let level = raw
                .parse::<u8>()
                .map_err(|_| anyhow::anyhow!("invalid relative strike: {value}"))?;
            Self::validate_level(prefix, level)
        };

        if normalized.starts_with("ITM") {
            return Ok(Self::Itm(parse_level("ITM")?));
        }
        if normalized.starts_with("OTM") {
            return Ok(Self::Otm(parse_level("OTM")?));
        }
        bail!("invalid relative strike: {value}")
    }
}

impl Serialize for RelativeStrike {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RelativeStrike {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

/// Option contract kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OptionType {
    Ce,
    Pe,
}

impl fmt::Display for OptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ce => "CE",
            Self::Pe => "PE",
        })
    }
}

/// Opening action for an option leg.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OptionAction {
    Buy,
    Sell,
}

impl OptionAction {
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

impl fmt::Display for OptionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        })
    }
}

/// OpenAlgo option order price type.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum OptionPriceType {
    #[default]
    #[serde(rename = "MARKET")]
    Market,
    #[serde(rename = "LIMIT")]
    Limit,
    #[serde(rename = "SL")]
    StopLimit,
    #[serde(rename = "SL-M")]
    StopMarket,
}

/// OpenAlgo option product.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OptionProduct {
    #[default]
    Mis,
    Nrml,
}

impl fmt::Display for OptionProduct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mis => "MIS",
            Self::Nrml => "NRML",
        })
    }
}

/// Behavior when OpenAlgo accepts only part of a multi-leg submission.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartialSuccessPolicy {
    #[default]
    KeepAccepted,
    CloseAccepted,
}

/// User-defined relative option leg.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OptionLegSpec {
    pub id: String,
    pub option_type: OptionType,
    pub strike: RelativeStrike,
    pub action: OptionAction,
    pub quantity: i32,
    pub expiry_date: String,
    #[serde(default)]
    pub splitsize: i32,
    #[serde(default)]
    pub pricetype: OptionPriceType,
    #[serde(default)]
    pub product: OptionProduct,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<f64>,
    #[serde(default)]
    pub disclosed_quantity: i32,
}

impl OptionLegSpec {
    #[must_use]
    pub fn market(
        id: impl Into<String>,
        option_type: OptionType,
        strike: RelativeStrike,
        action: OptionAction,
        quantity: i32,
        expiry_date: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            option_type,
            strike,
            action,
            quantity,
            expiry_date: normalize_expiry(&expiry_date.into()),
            splitsize: 0,
            pricetype: OptionPriceType::Market,
            product: OptionProduct::Mis,
            price: None,
            trigger_price: None,
            disclosed_quantity: 0,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(!self.id.trim().is_empty(), "leg id cannot be empty");
        ensure!(
            self.quantity > 0,
            "leg {} quantity must be positive",
            self.id
        );
        ensure!(
            !self.expiry_date.trim().is_empty(),
            "leg {} expiry_date cannot be empty",
            self.id
        );
        ensure!(
            self.splitsize >= 0,
            "leg {} splitsize cannot be negative",
            self.id
        );
        ensure!(
            self.disclosed_quantity >= 0,
            "leg {} disclosed_quantity cannot be negative",
            self.id
        );
        if self.pricetype == OptionPriceType::Limit {
            ensure!(
                self.price.is_some_and(|price| price > 0.0),
                "leg {} LIMIT order requires a positive price",
                self.id
            );
        }
        if matches!(
            self.pricetype,
            OptionPriceType::StopLimit | OptionPriceType::StopMarket
        ) {
            ensure!(
                self.trigger_price.is_some_and(|price| price > 0.0),
                "leg {} stop order requires a positive trigger_price",
                self.id
            );
        }
        Ok(())
    }
}

/// A generic relative-options submission for one underlying.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RelativeOptionsOrder {
    pub strategy: String,
    pub underlying: String,
    pub exchange: String,
    pub options_exchange: String,
    pub legs: Vec<OptionLegSpec>,
    #[serde(default)]
    pub partial_success: PartialSuccessPolicy,
}

impl RelativeOptionsOrder {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(!self.strategy.trim().is_empty(), "strategy cannot be empty");
        ensure!(
            !self.underlying.trim().is_empty(),
            "underlying cannot be empty"
        );
        ensure!(!self.exchange.trim().is_empty(), "exchange cannot be empty");
        ensure!(
            !self.options_exchange.trim().is_empty(),
            "options_exchange cannot be empty"
        );
        ensure!(
            (1..=20).contains(&self.legs.len()),
            "options order must contain 1 to 20 legs"
        );
        for leg in &self.legs {
            leg.validate()?;
        }
        for (index, leg) in self.legs.iter().enumerate() {
            ensure!(
                !self.legs[..index]
                    .iter()
                    .any(|previous| previous.id == leg.id),
                "duplicate leg id: {}",
                leg.id
            );
        }
        Ok(())
    }
}

/// Exact contract selected during a non-trading preview.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedOptionLeg {
    pub id: String,
    pub requested_strike: RelativeStrike,
    pub option_type: OptionType,
    pub expiry_date: String,
    pub symbol: String,
    pub exchange: String,
    pub lot_size: i32,
    pub tick_size: Option<f64>,
    pub freeze_quantity: Option<i32>,
    pub underlying_ltp: Option<f64>,
}

/// State of an exact contract accepted by OpenAlgo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookedLegState {
    Open,
    Closed { order_id: String },
    CloseRejected,
}

/// Exact contract identity returned by OpenAlgo after submission.
#[derive(Clone, Debug, PartialEq)]
pub struct BookedOptionLeg {
    pub id: String,
    pub requested_strike: RelativeStrike,
    pub option_type: OptionType,
    pub opening_action: OptionAction,
    pub quantity: i32,
    pub product: OptionProduct,
    pub expiry_date: String,
    pub symbol: String,
    pub exchange: String,
    pub opening_order_id: String,
    pub state: BookedLegState,
}

/// A leg rejected during a multi-leg submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedOptionLeg {
    pub id: String,
    pub status: String,
    pub message: Option<String>,
}

/// Result of submitting arbitrary relative option legs.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionsBooking {
    pub strategy: String,
    pub underlying: String,
    pub underlying_ltp: Option<f64>,
    pub mode: Option<String>,
    pub legs: Vec<BookedOptionLeg>,
    pub rejected: Vec<RejectedOptionLeg>,
    pub recovery_error: Option<String>,
}

impl OptionsBooking {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.rejected.is_empty()
    }

    #[must_use]
    pub fn has_open_legs(&self) -> bool {
        self.legs
            .iter()
            .any(|leg| !matches!(leg.state, BookedLegState::Closed { .. }))
    }
}

/// Exact-symbol close result.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OptionsCloseOutcome {
    pub closed: Vec<String>,
    pub rejected: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OptionsMultiOrderRequest {
    pub apikey: String,
    pub strategy: String,
    pub underlying: String,
    pub exchange: String,
    pub legs: Vec<OptionsMultiOrderLegRequest>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OptionsMultiOrderLegRequest {
    pub offset: String,
    pub option_type: String,
    pub action: String,
    pub quantity: i32,
    pub expiry_date: String,
    pub splitsize: i32,
    pub pricetype: OptionPriceType,
    pub product: OptionProduct,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<f64>,
    pub disclosed_quantity: i32,
}

impl From<&OptionLegSpec> for OptionsMultiOrderLegRequest {
    fn from(value: &OptionLegSpec) -> Self {
        Self {
            offset: value.strike.to_string(),
            option_type: value.option_type.to_string(),
            action: value.action.to_string(),
            quantity: value.quantity,
            expiry_date: normalize_expiry(&value.expiry_date),
            splitsize: value.splitsize,
            pricetype: value.pricetype,
            product: value.product,
            price: value.price,
            trigger_price: value.trigger_price,
            disclosed_quantity: value.disclosed_quantity,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OptionsMultiOrderResponse {
    pub status: String,
    pub underlying: Option<String>,
    pub underlying_ltp: Option<f64>,
    pub mode: Option<String>,
    pub results: Option<Vec<OptionsMultiOrderResult>>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OptionsMultiOrderResult {
    pub leg: i32,
    pub symbol: Option<String>,
    pub exchange: Option<String>,
    pub offset: Option<String>,
    pub option_type: Option<String>,
    pub action: Option<String>,
    pub status: String,
    pub orderid: Option<String>,
    pub message: Option<String>,
}

pub(crate) fn normalize_expiry(value: &str) -> String {
    value.trim().replace('-', "").to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bracket_and_openalgo_relative_strikes() {
        assert_eq!(
            "[atm-0]".parse::<RelativeStrike>().unwrap(),
            RelativeStrike::Atm
        );
        assert_eq!(
            "OTM-3".parse::<RelativeStrike>().unwrap(),
            RelativeStrike::Otm(3)
        );
        assert_eq!(
            "itm2".parse::<RelativeStrike>().unwrap(),
            RelativeStrike::Itm(2)
        );
        assert_eq!(RelativeStrike::Otm(3).to_string(), "OTM3");
    }

    #[test]
    fn rejects_invalid_relative_strike_levels() {
        assert!("OTM0".parse::<RelativeStrike>().is_err());
        assert!("ITM51".parse::<RelativeStrike>().is_err());
        assert!("ATM-1".parse::<RelativeStrike>().is_err());
    }
}
