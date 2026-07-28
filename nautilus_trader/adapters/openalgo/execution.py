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

from __future__ import annotations

import asyncio
from datetime import datetime
from decimal import Decimal
from typing import Any

from nautilus_trader.adapters.openalgo.config import OpenAlgoExecClientConfig
from nautilus_trader.adapters.openalgo.constants import OPENALGO_CLIENT_ID
from nautilus_trader.adapters.openalgo.http.client import OpenAlgoHttpClient
from nautilus_trader.adapters.openalgo.providers import OpenAlgoInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.enums import LogColor
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.execution.messages import BatchCancelOrders
from nautilus_trader.execution.messages import CancelAllOrders
from nautilus_trader.execution.messages import CancelOrder
from nautilus_trader.execution.messages import GenerateFillReports
from nautilus_trader.execution.messages import GenerateOrderStatusReport
from nautilus_trader.execution.messages import GenerateOrderStatusReports
from nautilus_trader.execution.messages import GeneratePositionStatusReports
from nautilus_trader.execution.messages import ModifyOrder
from nautilus_trader.execution.messages import QueryAccount
from nautilus_trader.execution.messages import SubmitOrder
from nautilus_trader.execution.messages import SubmitOrderList
from nautilus_trader.execution.reports import FillReport
from nautilus_trader.execution.reports import OrderStatusReport
from nautilus_trader.execution.reports import PositionStatusReport
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import LiquiditySide
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import OrderStatus
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import PositionSide
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.identifiers import AccountId
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import TradeId
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.identifiers import VenueOrderId
from nautilus_trader.model.objects import Currency
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.model.orders import Order


class OpenAlgoExecutionClient(LiveExecutionClient):
    """
    Provides an execution client for OpenAlgo broker integrations.
    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client: OpenAlgoHttpClient,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: OpenAlgoInstrumentProvider,
        config: OpenAlgoExecClientConfig,
        name: str | None = None,
    ) -> None:
        venue = Venue(config.venue)
        base_currency = Currency.from_str(config.base_currency)

        super().__init__(
            loop=loop,
            client_id=ClientId(name or OPENALGO_CLIENT_ID.value),
            venue=venue,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=base_currency,
            instrument_provider=instrument_provider,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
        )

        self._client = client
        self._config = config
        self._instrument_provider = instrument_provider
        self._base_currency = base_currency

        account_id = AccountId(config.account_id or f"{OPENALGO_CLIENT_ID.value}-{venue.value}")
        self._set_account_id(account_id)

        self._log.info(f"config.base_url_http={config.base_url_http}", LogColor.BLUE)
        self._log.info(f"config.base_url_ws={config.base_url_ws}", LogColor.BLUE)
        self._log.info(f"config.venue={config.venue}", LogColor.BLUE)
        self._log.info(f"config.product={config.product}", LogColor.BLUE)

    async def _connect(self) -> None:
        self._log.info("Connecting to OpenAlgo...", LogColor.BLUE)
        await self._client.connect()
        await self._instrument_provider.initialize()
        self._log.info("OpenAlgo execution client connected", LogColor.GREEN)

    async def _disconnect(self) -> None:
        await self._client.close()
        self._log.info("OpenAlgo execution client disconnected", LogColor.GREEN)

    async def _submit_order(self, command: SubmitOrder) -> None:
        order = command.order
        if order.is_closed:
            self._log.warning(f"Order {order} is already closed")
            return

        self.generate_order_submitted(
            strategy_id=order.strategy_id,
            instrument_id=order.instrument_id,
            client_order_id=order.client_order_id,
            ts_event=self._clock.timestamp_ns(),
        )

        try:
            response = await self._client.place_order(
                strategy=self._strategy_name(order),
                symbol=self._symbol(order.instrument_id),
                action=self._action(order.side),
                exchange=self._exchange(order.instrument_id),
                pricetype=self._price_type(order.order_type),
                product=self._product(order),
                quantity=self._quantity(order.quantity),
                price=self._price(order.price if order.has_price else None),
                trigger_price=self._price(
                    order.trigger_price if order.has_trigger_price else None,
                ),
            )
            venue_order_id = VenueOrderId(str(response["orderid"]))
            self.generate_order_accepted(
                strategy_id=order.strategy_id,
                instrument_id=order.instrument_id,
                client_order_id=order.client_order_id,
                venue_order_id=venue_order_id,
                ts_event=self._clock.timestamp_ns(),
            )
        except Exception as e:
            self._log.error(f"Error submitting order {order.client_order_id}: {e}")
            self.generate_order_rejected(
                strategy_id=order.strategy_id,
                instrument_id=order.instrument_id,
                client_order_id=order.client_order_id,
                reason=str(e),
                ts_event=self._clock.timestamp_ns(),
            )

    async def _submit_order_list(self, command: SubmitOrderList) -> None:
        for order in command.order_list.orders:
            await self._submit_order(
                SubmitOrder(
                    trader_id=command.trader_id,
                    strategy_id=command.strategy_id,
                    order=order,
                    command_id=command.id,
                    ts_init=command.ts_init,
                    client_id=command.client_id,
                    position_id=command.position_id,
                    params=command.params,
                ),
            )

    async def _modify_order(self, command: ModifyOrder) -> None:
        order = self._cache.order(command.client_order_id)
        venue_order_id = command.venue_order_id or self._cache.venue_order_id(
            command.client_order_id,
        )
        if order is None or venue_order_id is None:
            self.generate_order_modify_rejected(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=venue_order_id or VenueOrderId("UNKNOWN"),
                reason="Missing cached order or venue_order_id",
                ts_event=self._clock.timestamp_ns(),
            )
            return

        quantity = command.quantity or order.quantity
        price = command.price or (order.price if order.has_price else None)
        trigger_price = command.trigger_price or (
            order.trigger_price if order.has_trigger_price else None
        )

        try:
            await self._client.modify_order(
                strategy=self._strategy_name(order),
                orderid=venue_order_id.value,
                symbol=self._symbol(command.instrument_id),
                action=self._action(order.side),
                exchange=self._exchange(command.instrument_id),
                pricetype=self._price_type(order.order_type),
                product=self._product(order),
                quantity=self._quantity(quantity),
                price=self._price(price),
                trigger_price=self._price(trigger_price),
            )
            self.generate_order_updated(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=venue_order_id,
                quantity=quantity,
                price=price,
                trigger_price=trigger_price,
                ts_event=self._clock.timestamp_ns(),
            )
        except Exception as e:
            self._log.error(f"Error modifying order {command.client_order_id}: {e}")
            self.generate_order_modify_rejected(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=venue_order_id,
                reason=str(e),
                ts_event=self._clock.timestamp_ns(),
            )

    async def _cancel_order(self, command: CancelOrder) -> None:
        venue_order_id = command.venue_order_id or self._cache.venue_order_id(
            command.client_order_id,
        )
        if venue_order_id is None:
            self.generate_order_cancel_rejected(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=VenueOrderId("UNKNOWN"),
                reason="Missing venue_order_id",
                ts_event=self._clock.timestamp_ns(),
            )
            return

        try:
            await self._client.cancel_order(
                strategy=self._config.strategy,
                orderid=venue_order_id.value,
            )
            self.generate_order_canceled(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=venue_order_id,
                ts_event=self._clock.timestamp_ns(),
            )
        except Exception as e:
            self._log.error(f"Error canceling order {command.client_order_id}: {e}")
            self.generate_order_cancel_rejected(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=venue_order_id,
                reason=str(e),
                ts_event=self._clock.timestamp_ns(),
            )

    async def _cancel_all_orders(self, command: CancelAllOrders) -> None:
        try:
            await self._client.cancel_all_order(strategy=self._config.strategy)
        except Exception as e:
            self._log.error(f"Error canceling all orders: {e}")

    async def _batch_cancel_orders(self, command: BatchCancelOrders) -> None:
        for cancel in command.cancels:
            await self._cancel_order(cancel)

    async def generate_order_status_report(
        self,
        command: GenerateOrderStatusReport,
    ) -> OrderStatusReport | None:
        venue_order_id = command.venue_order_id
        if venue_order_id is None and command.client_order_id is not None:
            venue_order_id = self._cache.venue_order_id(command.client_order_id)
        if venue_order_id is None:
            self._log.warning("Cannot query OpenAlgo order status without venue_order_id")
            return None

        response = await self._client.order_status(
            strategy=self._config.strategy,
            orderid=venue_order_id.value,
        )
        data = response.get("data") or {}
        return self._parse_order_status_report(
            data=data,
            client_order_id=command.client_order_id,
            instrument_id=command.instrument_id,
        )

    async def generate_order_status_reports(
        self,
        command: GenerateOrderStatusReports,
    ) -> list[OrderStatusReport]:
        response = await self._client.orderbook()
        orders = (response.get("data") or {}).get("orders") or []
        reports = [
            report
            for item in orders
            if (report := self._parse_order_status_report(item, None, command.instrument_id))
            is not None
        ]
        self._log_report_receipt(
            len(reports),
            "OrderStatusReport",
            command.log_receipt_level,
            "Generated",
        )
        return reports

    async def generate_fill_reports(
        self,
        command: GenerateFillReports,
    ) -> list[FillReport]:
        response = await self._client.tradebook()
        trades = response.get("data") or []
        reports = [
            report
            for item in trades
            if (report := self._parse_fill_report(item, command.instrument_id)) is not None
        ]
        self._log_report_receipt(len(reports), "FillReport", command.log_receipt_level, "Generated")
        return reports

    async def generate_position_status_reports(
        self,
        command: GeneratePositionStatusReports,
    ) -> list[PositionStatusReport]:
        response = await self._client.positionbook()
        positions = response.get("data") or []
        reports = [
            report
            for item in positions
            if (report := self._parse_position_status_report(item, command.instrument_id))
            is not None
        ]
        self._log_report_receipt(
            len(reports),
            "PositionStatusReport",
            command.log_receipt_level,
            "Generated",
        )
        return reports

    async def _query_account(self, command: QueryAccount) -> None:
        response = await self._client.funds()
        self._log.info(f"OpenAlgo funds response: {response.get('data')}")

    def _parse_order_status_report(
        self,
        data: dict[str, Any],
        client_order_id: Any | None,
        instrument_id: InstrumentId | None,
    ) -> OrderStatusReport | None:
        if not data:
            return None

        instrument_id = instrument_id or self._instrument_id(data)
        if instrument_id is None:
            return None

        order_status = self._order_status(data.get("order_status"))
        quantity = self._make_qty(instrument_id, data.get("quantity", "0"))
        filled_qty = quantity if order_status == OrderStatus.FILLED else Quantity.zero(
            quantity.precision,
        )
        ts_last = self._timestamp_ns(data.get("timestamp"))

        return OrderStatusReport(
            account_id=self.account_id,
            instrument_id=instrument_id,
            client_order_id=client_order_id,
            order_list_id=None,
            venue_order_id=VenueOrderId(str(data["orderid"])),
            order_side=self._order_side(data.get("action")),
            order_type=self._order_type(data.get("pricetype")),
            time_in_force=TimeInForce.DAY,
            order_status=order_status,
            price=self._make_price_or_none(instrument_id, data.get("price")),
            trigger_price=self._make_price_or_none(instrument_id, data.get("trigger_price")),
            quantity=quantity,
            filled_qty=filled_qty,
            avg_px=self._decimal_or_none(data.get("average_price")),
            ts_accepted=ts_last,
            ts_last=ts_last,
            report_id=UUID4(),
            ts_init=self._clock.timestamp_ns(),
        )

    def _parse_fill_report(
        self,
        data: dict[str, Any],
        instrument_id: InstrumentId | None,
    ) -> FillReport | None:
        instrument_id = instrument_id or self._instrument_id(data)
        if instrument_id is None:
            return None

        ts_event = self._timestamp_ns(data.get("timestamp"))
        return FillReport(
            account_id=self.account_id,
            instrument_id=instrument_id,
            client_order_id=None,
            venue_order_id=VenueOrderId(str(data["orderid"])),
            trade_id=TradeId(f"{data['orderid']}-{ts_event}"),
            order_side=self._order_side(data.get("action")),
            last_qty=self._make_qty(instrument_id, data.get("quantity", "0")),
            last_px=self._make_price(instrument_id, data.get("average_price", "0")),
            commission=Money(0, self._base_currency),
            liquidity_side=LiquiditySide.NO_LIQUIDITY_SIDE,
            report_id=UUID4(),
            ts_event=ts_event,
            ts_init=self._clock.timestamp_ns(),
        )

    def _parse_position_status_report(
        self,
        data: dict[str, Any],
        instrument_id: InstrumentId | None,
    ) -> PositionStatusReport | None:
        instrument_id = instrument_id or self._instrument_id(data)
        if instrument_id is None:
            return None

        raw_qty = Decimal(str(data.get("quantity", "0")))
        side = PositionSide.FLAT
        if raw_qty > 0:
            side = PositionSide.LONG
        elif raw_qty < 0:
            side = PositionSide.SHORT

        return PositionStatusReport(
            account_id=self.account_id,
            instrument_id=instrument_id,
            position_side=side,
            quantity=self._make_qty(instrument_id, abs(raw_qty)),
            avg_px_open=self._decimal_or_none(data.get("average_price")),
            report_id=UUID4(),
            ts_last=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )

    def _instrument_id(self, data: dict[str, Any]) -> InstrumentId | None:
        symbol = data.get("symbol")
        exchange = data.get("exchange") or self._config.venue
        if not symbol:
            return None
        return InstrumentId.from_str(f"{symbol}.{exchange}")

    def _symbol(self, instrument_id: InstrumentId) -> str:
        return instrument_id.symbol.value

    def _exchange(self, instrument_id: InstrumentId) -> str:
        return instrument_id.venue.value

    def _strategy_name(self, order: Order) -> str:
        params = order.params or {}
        return str(params.get("strategy", self._config.strategy))

    def _product(self, order: Order) -> str:
        params = order.params or {}
        return str(params.get("product", self._config.product))

    def _quantity(self, quantity: Quantity) -> str:
        return quantity.to_formatted_str()

    def _price(self, price: Price | None) -> str:
        return price.to_formatted_str() if price is not None else "0"

    def _action(self, side: OrderSide) -> str:
        if side == OrderSide.BUY:
            return "BUY"
        if side == OrderSide.SELL:
            return "SELL"
        raise ValueError(f"Unsupported OpenAlgo order side: {side}")

    def _price_type(self, order_type: OrderType) -> str:
        if order_type == OrderType.MARKET:
            return "MARKET"
        if order_type == OrderType.LIMIT:
            return "LIMIT"
        if order_type == OrderType.STOP_MARKET:
            return "SL-M"
        if order_type == OrderType.STOP_LIMIT:
            return "SL"
        raise ValueError(f"Unsupported OpenAlgo order type: {order_type}")

    def _order_side(self, value: Any) -> OrderSide:
        return OrderSide.BUY if str(value).upper() == "BUY" else OrderSide.SELL

    def _order_type(self, value: Any) -> OrderType:
        value = str(value).upper()
        if value == "LIMIT":
            return OrderType.LIMIT
        if value == "SL":
            return OrderType.STOP_LIMIT
        if value == "SL-M":
            return OrderType.STOP_MARKET
        return OrderType.MARKET

    def _order_status(self, value: Any) -> OrderStatus:
        value = str(value).lower()
        if value in {"complete", "completed", "filled"}:
            return OrderStatus.FILLED
        if value in {"cancelled", "canceled"}:
            return OrderStatus.CANCELED
        if value in {"rejected", "reject"}:
            return OrderStatus.REJECTED
        if value in {"trigger pending", "trigger_pending"}:
            return OrderStatus.TRIGGERED
        return OrderStatus.ACCEPTED

    def _make_qty(self, instrument_id: InstrumentId, value: Any) -> Quantity:
        instrument = self._cache.instrument(instrument_id) or self._instrument_provider.find(
            instrument_id,
        )
        if instrument is not None:
            return instrument.make_qty(value)
        return Quantity.from_str(str(value))

    def _make_price(self, instrument_id: InstrumentId, value: Any) -> Price:
        instrument = self._cache.instrument(instrument_id) or self._instrument_provider.find(
            instrument_id,
        )
        if instrument is not None:
            return instrument.make_price(value)
        return Price.from_str(str(value))

    def _make_price_or_none(self, instrument_id: InstrumentId, value: Any) -> Price | None:
        decimal = self._decimal_or_none(value)
        if decimal is None or decimal == 0:
            return None
        return self._make_price(instrument_id, decimal)

    def _decimal_or_none(self, value: Any) -> Decimal | None:
        if value is None or value == "":
            return None
        return Decimal(str(value))

    def _timestamp_ns(self, value: Any) -> int:
        if not value:
            return self._clock.timestamp_ns()
        try:
            dt = datetime.strptime(str(value), "%d-%b-%Y %H:%M:%S")
            return int(dt.timestamp() * 1_000_000_000)
        except ValueError:
            return self._clock.timestamp_ns()
