#!/usr/bin/env python3
# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Example: EMA(9)/EMA(20) crossover backtest for RELIANCE on NSE.

The example uses deterministic synthetic quotes so it can run without external
market data or credentials. The instrument identifiers match the OpenAlgo
adapter convention: symbol `RELIANCE`, exchange `NSE`.
"""

from decimal import Decimal

import pandas as pd

from nautilus_trader.backtest.config import BacktestEngineConfig
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.examples.strategies.ema_cross import EMACross
from nautilus_trader.examples.strategies.ema_cross import EMACrossConfig
from nautilus_trader.model.currencies import INR
from nautilus_trader.model.data import BarType
from nautilus_trader.model.data import QuoteTick
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import BookType
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity


FAST_EMA_PERIOD = 9
SLOW_EMA_PERIOD = 20

BASE_TS_NS = 1_735_689_600_000_000_000  # 2025-01-01T00:00:00Z
INTERVAL_NS = 60_000_000_000  # One minute
SPREAD = 0.10


def reliance_equity() -> Equity:
    return Equity(
        instrument_id=InstrumentId.from_str("RELIANCE.NSE"),
        raw_symbol=Symbol("RELIANCE"),
        currency=INR,
        price_precision=2,
        price_increment=Price.from_str("0.05"),
        lot_size=Quantity.from_int(1),
        ts_event=0,
        ts_init=0,
        isin="INE002A01018",
    )


def quote(instrument_id: InstrumentId, mid: float, tick: int) -> QuoteTick:
    ts = BASE_TS_NS + tick * INTERVAL_NS
    return QuoteTick(
        instrument_id=instrument_id,
        bid_price=Price(mid - SPREAD / 2.0, precision=2),
        ask_price=Price(mid + SPREAD / 2.0, precision=2),
        bid_size=Quantity.from_int(10_000),
        ask_size=Quantity.from_int(10_000),
        ts_event=ts,
        ts_init=ts,
    )


def generate_quotes(instrument_id: InstrumentId) -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    tick = 0

    def add(mid: float) -> None:
        nonlocal tick
        quotes.append(quote(instrument_id, mid, tick))
        tick += 1

    # Initialize both EMAs at the same price.
    for _ in range(25):
        add(2_500.00)

    # Fast EMA crosses above slow EMA and enters a long position.
    for i in range(40):
        add(2_500.00 + i * 5.00)

    # Fast EMA crosses below slow EMA and exits the long position.
    for i in range(80):
        add(2_695.00 - i * 5.00)

    return quotes


if __name__ == "__main__":
    engine = BacktestEngine(config=BacktestEngineConfig())

    NSE = Venue("NSE")
    engine.add_venue(
        venue=NSE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        base_currency=INR,
        starting_balances=[Money(1_000_000, INR)],
        book_type=BookType.L1_MBP,
    )

    instrument = reliance_equity()
    instrument_id = instrument.id
    engine.add_instrument(instrument)

    quotes = generate_quotes(instrument_id)
    engine.add_data(quotes)

    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=BarType.from_str(f"{instrument_id}-1-MINUTE-MID-INTERNAL"),
        trade_size=Decimal(1),
        fast_ema_period=FAST_EMA_PERIOD,
        slow_ema_period=SLOW_EMA_PERIOD,
    )
    engine.add_strategy(EMACross(config=strategy_config))

    engine.run()

    with pd.option_context(
        "display.max_rows",
        100,
        "display.max_columns",
        None,
        "display.width",
        300,
    ):
        print(engine.trader.generate_account_report(NSE))
        print(engine.trader.generate_order_fills_report())
        print(engine.trader.generate_positions_report())

    result = engine.get_result()
    print(f"RELIANCE.NSE EMA({FAST_EMA_PERIOD})/EMA({SLOW_EMA_PERIOD}) backtest complete")
    print(f"Quotes processed: {len(quotes)}")
    print(f"Orders generated: {result.total_orders}")
    print(f"Closed positions: {result.total_positions}")

    engine.reset()
    engine.dispose()
