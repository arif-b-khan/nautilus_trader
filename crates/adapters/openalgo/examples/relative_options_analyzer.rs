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

use std::env;

use anyhow::{Context, Result, ensure};
use nautilus_openalgo::{
    OpenAlgoHttpClient, OpenAlgoOptionsAdapter, OptionLegSpec, PartialSuccessPolicy,
    RelativeOptionsOrder,
};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = required_env("OPENALGO_API_KEY")?;
    let host = required_env("OPENALGO_HOST")?;
    let legs = serde_json::from_str::<Vec<OptionLegSpec>>(&required_env("OPENALGO_OPTION_LEGS")?)
        .context("OPENALGO_OPTION_LEGS must be a JSON array of option legs")?;
    let order = RelativeOptionsOrder {
        strategy: required_env("OPENALGO_STRATEGY")?,
        underlying: required_env("OPENALGO_UNDERLYING")?,
        exchange: required_env("OPENALGO_UNDERLYING_EXCHANGE")?,
        options_exchange: required_env("OPENALGO_OPTIONS_EXCHANGE")?,
        legs,
        partial_success: PartialSuccessPolicy::CloseAccepted,
    };

    let client = OpenAlgoHttpClient::new(&api_key, &host, "v1", "");
    let adapter = OpenAlgoOptionsAdapter::new(client.clone());
    for leg in adapter.preview(&order).await? {
        println!(
            "Preview {}: {} {} -> {}.{}",
            leg.id, leg.option_type, leg.requested_strike, leg.symbol, leg.exchange
        );
    }

    let analyzer = client.analyzer_status().await?;
    ensure!(
        analyzer
            .data
            .as_ref()
            .and_then(|data| data.analyze_mode)
            .unwrap_or(false),
        "OpenAlgo is not in analyzer mode; refusing to submit"
    );
    ensure!(
        env::var("OPENALGO_CONFIRM_ANALYZER_ORDER").as_deref() == Ok("yes"),
        "set OPENALGO_CONFIRM_ANALYZER_ORDER=yes to submit the analyzer order"
    );

    let booking = adapter.submit(&order).await?;
    for leg in &booking.legs {
        println!(
            "Booked {}: {} {} -> {}.{}, order {}",
            leg.id,
            leg.option_type,
            leg.requested_strike,
            leg.symbol,
            leg.exchange,
            leg.opening_order_id
        );
    }
    for leg in &booking.rejected {
        println!("Rejected {}: {}", leg.id, leg.status);
    }
    if let Some(error) = booking.recovery_error {
        anyhow::bail!("partial-submission recovery failed: {error}");
    }
    Ok(())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("{name} is required"))
}
