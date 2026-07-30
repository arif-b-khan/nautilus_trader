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
Self-contained NIFTY long-straddle backtest using OpenAlgo relative strike selectors.

The strategy resolves both ``[atm-0]`` legs from Nautilus option instruments at
entry time. It then retains and exits those exact contracts after NIFTY moves to
a new ATM strike. No OpenAlgo network request or API key is used in backtesting.
"""

from __future__ import annotations

from datetime import UTC
from datetime import datetime
from datetime import timedelta

from nautilus_trader.adapters.openalgo.options import resolve_openalgo_option
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.engine import BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.core.datetime import dt_to_unix_nanos
from nautilus_trader.model.data import QuoteTick
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import AssetClass
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.enums import OptionKind
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import PriceType
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.instruments import IndexInstrument
from nautilus_trader.model.instruments import OptionContract
from nautilus_trader.model.objects import Currency
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.trading.strategy import Strategy


NSE_INDEX = Venue("NSE_INDEX")
NFO = Venue("NFO")
INR = Currency.from_str("INR")
NIFTY_ID = InstrumentId.from_str("NIFTY.NSE_INDEX")
START = datetime(2026, 7, 30, 3, 45, tzinfo=UTC)  # 09:15 Asia/Kolkata
EXPIRY = datetime(2026, 8, 6, 10, 0, tzinfo=UTC)
EXPIRY_NS = dt_to_unix_nanos(EXPIRY)
STRIKES = (24_900, 25_000, 25_100)


class OpenAlgoLongStraddleConfig(StrategyConfig, frozen=True):
    underlying_id: InstrumentId
    option_venue: Venue
    expiration_ns: int
    call_strike: str = "[atm-0]"
    put_strike: str = "[atm-0]"
    quantity: int = 75
    exit_after_underlying_quotes: int = 5


class OpenAlgoLongStraddleBacktest(Strategy):
    """
    Resolves moving OpenAlgo selectors once and trades the resulting exact contracts.
    """

    def __init__(self, config: OpenAlgoLongStraddleConfig) -> None:
        super().__init__(config=config)
        self.call_id: InstrumentId | None = None
        self.put_id: InstrumentId | None = None
        self._latest_underlying = 0.0
        self._selected_quotes: set[InstrumentId] = set()
        self._entry_submitted = False
        self._entry_fills = 0
        self._exit_submitted = False
        self._underlying_quotes = 0

    def on_start(self) -> None:
        self.subscribe_quote_ticks(self.config.underlying_id)
        for instrument in self.cache.instruments(
            venue=self.config.option_venue,
            underlying="NIFTY",
        ):
            self.subscribe_quote_ticks(instrument.id)

    def on_stop(self) -> None:
        self.unsubscribe_quote_ticks(self.config.underlying_id)
        for instrument in self.cache.instruments(
            venue=self.config.option_venue,
            underlying="NIFTY",
        ):
            self.unsubscribe_quote_ticks(instrument.id)

    def on_quote_tick(self, tick: QuoteTick) -> None:
        if tick.instrument_id == self.config.underlying_id:
            self._underlying_quotes += 1
            self._latest_underlying = tick.extract_price(PriceType.MID).as_double()
            if self.call_id is None:
                self._resolve_and_lock_contracts()
            if (
                self._entry_fills == 2
                and not self._exit_submitted
                and self._underlying_quotes >= self.config.exit_after_underlying_quotes
            ):
                self._submit_exit()
            return

        if tick.instrument_id in {self.call_id, self.put_id}:
            self._selected_quotes.add(tick.instrument_id)
            if (
                not self._entry_submitted
                and self.call_id in self._selected_quotes
                and self.put_id in self._selected_quotes
            ):
                self._submit_entry()

    def on_order_filled(self, event) -> None:
        if not self._exit_submitted:
            self._entry_fills += 1
        self.log.warning(
            f"Fill {event.instrument_id}: {event.order_side} {event.last_qty} @ {event.last_px}",
        )

    def _resolve_and_lock_contracts(self) -> None:
        instruments = self.cache.instruments(
            venue=self.config.option_venue,
            underlying="NIFTY",
        )
        call = resolve_openalgo_option(
            instruments,
            underlying_price=self._latest_underlying,
            option_kind=OptionKind.CALL,
            strike_selector=self.config.call_strike,
            expiration_ns=self.config.expiration_ns,
        )
        put = resolve_openalgo_option(
            instruments,
            underlying_price=self._latest_underlying,
            option_kind=OptionKind.PUT,
            strike_selector=self.config.put_strike,
            expiration_ns=self.config.expiration_ns,
        )
        self.call_id = call.id
        self.put_id = put.id
        self.log.warning(
            f"Locked selectors at NIFTY={self._latest_underlying:.2f}: "
            f"{self.call_id}, {self.put_id}",
        )

    def _submit_entry(self) -> None:
        assert self.call_id is not None
        assert self.put_id is not None
        self._entry_submitted = True
        for instrument_id in (self.call_id, self.put_id):
            order = self.order_factory.market(
                instrument_id=instrument_id,
                order_side=OrderSide.BUY,
                quantity=Quantity.from_int(self.config.quantity),
            )
            self.submit_order(order)

    def _submit_exit(self) -> None:
        assert self.call_id is not None
        assert self.put_id is not None
        self._exit_submitted = True
        self.log.warning(
            f"NIFTY moved to {self._latest_underlying:.2f}; closing locked "
            f"contracts {self.call_id}, {self.put_id}",
        )
        for instrument_id in (self.call_id, self.put_id):
            order = self.order_factory.market(
                instrument_id=instrument_id,
                order_side=OrderSide.SELL,
                quantity=Quantity.from_int(self.config.quantity),
            )
            self.submit_order(order)


def create_nifty_index() -> IndexInstrument:
    return IndexInstrument(
        instrument_id=NIFTY_ID,
        raw_symbol=Symbol("NIFTY"),
        currency=INR,
        price_precision=2,
        size_precision=0,
        price_increment=Price.from_str("0.05"),
        size_increment=Quantity.from_int(1),
        ts_event=0,
        ts_init=0,
    )


def create_option(strike: int, kind: OptionKind) -> OptionContract:
    suffix = "CE" if kind is OptionKind.CALL else "PE"
    symbol = f"NIFTY06AUG26{strike}{suffix}"
    return OptionContract(
        instrument_id=InstrumentId.from_str(f"{symbol}.NFO"),
        raw_symbol=Symbol(symbol),
        asset_class=AssetClass.INDEX,
        underlying="NIFTY",
        option_kind=kind,
        strike_price=Price.from_str(f"{strike}.00"),
        currency=INR,
        activation_ns=dt_to_unix_nanos(START - timedelta(days=30)),
        expiration_ns=EXPIRY_NS,
        price_precision=2,
        price_increment=Price.from_str("0.05"),
        multiplier=Quantity.from_int(1),
        lot_size=Quantity.from_int(75),
        ts_event=0,
        ts_init=0,
        exchange="NFO",
    )


def quote(
    instrument_id: InstrumentId,
    bid: float,
    ask: float,
    timestamp_ns: int,
    size: int = 1_800,
) -> QuoteTick:
    return QuoteTick(
        instrument_id=instrument_id,
        bid_price=Price.from_str(f"{bid:.2f}"),
        ask_price=Price.from_str(f"{ask:.2f}"),
        bid_size=Quantity.from_int(size),
        ask_size=Quantity.from_int(size),
        ts_event=timestamp_ns,
        ts_init=timestamp_ns,
    )


def create_data(options: list[OptionContract]) -> list[QuoteTick]:
    underlying_prices = (25_020.0, 25_040.0, 25_080.0, 25_120.0, 25_140.0, 25_160.0)
    data: list[QuoteTick] = []

    for minute, underlying in enumerate(underlying_prices):
        timestamp = dt_to_unix_nanos(START + timedelta(minutes=minute))
        data.append(quote(NIFTY_ID, underlying - 0.5, underlying + 0.5, timestamp, size=1))

        for offset, instrument in enumerate(options, start=1):
            strike = instrument.strike_price.as_double()
            intrinsic = (
                max(underlying - strike, 0.0)
                if instrument.option_kind is OptionKind.CALL
                else max(strike - underlying, 0.0)
            )
            time_value = max(35.0, 105.0 - (minute * 8.0) - abs(strike - underlying) * 0.15)
            mid = intrinsic + time_value
            data.append(
                quote(
                    instrument.id,
                    mid - 0.5,
                    mid + 0.5,
                    timestamp + offset,
                ),
            )
    return data


def run_backtest() -> None:
    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id="OPENALGO-BACKTEST-001",
            logging=LoggingConfig(log_level="WARNING"),
        ),
    )
    engine.add_venue(
        venue=NSE_INDEX,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        base_currency=INR,
        starting_balances=[Money(1_000_000, INR)],
    )
    engine.add_venue(
        venue=NFO,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=INR,
        starting_balances=[Money(1_000_000, INR)],
    )

    index = create_nifty_index()
    options = [
        create_option(strike, kind)
        for strike in STRIKES
        for kind in (OptionKind.CALL, OptionKind.PUT)
    ]
    engine.add_instrument(index)
    for option in options:
        engine.add_instrument(option)
    engine.add_data(create_data(options))

    strategy = OpenAlgoLongStraddleBacktest(
        config=OpenAlgoLongStraddleConfig(
            underlying_id=index.id,
            option_venue=NFO,
            expiration_ns=EXPIRY_NS,
        ),
    )
    engine.add_strategy(strategy)
    engine.run()

    assert strategy.call_id == InstrumentId.from_str("NIFTY06AUG2625000CE.NFO")
    assert strategy.put_id == InstrumentId.from_str("NIFTY06AUG2625000PE.NFO")
    assert len(engine.cache.orders_closed(venue=NFO)) == 4
    assert not engine.cache.positions_open(venue=NFO)

    print(engine.trader.generate_order_fills_report())
    print(engine.trader.generate_positions_report())
    print(engine.trader.generate_account_report(NFO))
    engine.dispose()


if __name__ == "__main__":
    run_backtest()
