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

use openalgo::OptionGreeksResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MultiOptionGreeksInstrument {
    pub symbol: String,
    pub exchange: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlying_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlying_exchange: Option<String>,
}

impl MultiOptionGreeksInstrument {
    #[must_use]
    pub fn new(symbol: impl Into<String>, exchange: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            exchange: exchange.into(),
            underlying_symbol: None,
            underlying_exchange: None,
        }
    }

    #[must_use]
    pub fn with_underlying(
        mut self,
        symbol: impl Into<String>,
        exchange: impl Into<String>,
    ) -> Self {
        self.underlying_symbol = Some(symbol.into());
        self.underlying_exchange = Some(exchange.into());
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct MultiOptionGreeksRequest {
    pub apikey: String,
    pub symbols: Vec<MultiOptionGreeksInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_time: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MultiOptionGreeksSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MultiOptionGreeksResponse {
    pub status: String,
    pub data: Option<Vec<OptionGreeksResponse>>,
    pub summary: Option<MultiOptionGreeksSummary>,
    pub message: Option<String>,
}
