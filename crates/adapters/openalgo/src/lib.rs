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

//! [NautilusTrader](http://nautilustrader.io) adapter for [OpenAlgo](https://openalgo.in).

#![warn(rustc::all)]
#![deny(unsafe_code)]
#![deny(nonstandard_style)]
#![deny(missing_debug_implementations)]
#![deny(clippy::missing_panics_doc)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod http;
pub mod options;

#[cfg(feature = "python")]
pub mod python;

pub use crate::http::client::OpenAlgoHttpClient;
pub use crate::{
    http::models::{
        MultiOptionGreeksInstrument, MultiOptionGreeksResponse, MultiOptionGreeksSummary,
    },
    options::{
        execution::OpenAlgoOptionsAdapter,
        models::{
            BookedLegState, BookedOptionLeg, OptionAction, OptionLegSpec, OptionPriceType,
            OptionProduct, OptionType, OptionsBooking, OptionsCloseOutcome, PartialSuccessPolicy,
            RejectedOptionLeg, RelativeOptionsOrder, RelativeStrike, ResolvedOptionLeg,
        },
    },
};
pub use openalgo::{
    BasketOrderItem, BasketOrderResponse, ExpiryResponse, OptionChainResponse,
    OptionGreeksResponse, OptionSymbolResponse, OptionsLeg, OptionsMultiOrderResponse,
    OptionsOrderResponse, QuotesResponse, SyntheticFutureResponse,
};
