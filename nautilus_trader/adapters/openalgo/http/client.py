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

import json
from typing import Any

from nautilus_trader._libnautilus.openalgo import OpenAlgoHttpClient as RustOpenAlgoHttpClient


class OpenAlgoHttpError(RuntimeError):
    """
    Raised when the OpenAlgo HTTP API returns an error response.
    """


class OpenAlgoHttpClient:
    """
    Thin Python compatibility wrapper around the Rust OpenAlgo SDK client.
    """

    def __init__(
        self,
        api_key: str | None = None,
        base_url: str = "http://127.0.0.1:5000",
        api_version: str = "v1",
        ws_url: str = "ws://127.0.0.1:8765",
        timeout_secs: int = 10,
        proxy_url: str | None = None,
    ) -> None:
        if timeout_secs != 10 or proxy_url is not None:
            raise ValueError(
                "timeout_secs/proxy_url are not supported by the OpenAlgo Rust SDK client yet",
            )

        self._client = RustOpenAlgoHttpClient(
            api_key,
            base_url,
            api_version,
            ws_url,
            None,
            None,
        )

    async def connect(self) -> None:
        await self._client.connect()

    async def close(self) -> None:
        await self._client.close()

    async def funds(self) -> dict[str, Any]:
        return self._decode(await self._client.funds())

    async def orderbook(self) -> dict[str, Any]:
        return self._decode(await self._client.orderbook())

    async def tradebook(self) -> dict[str, Any]:
        return self._decode(await self._client.tradebook())

    async def positionbook(self) -> dict[str, Any]:
        return self._decode(await self._client.positionbook())

    async def place_order(
        self,
        *,
        strategy: str,
        symbol: str,
        action: str,
        exchange: str,
        pricetype: str,
        product: str,
        quantity: str,
        price: str = "0",
        trigger_price: str = "0",
        disclosed_quantity: str = "0",
    ) -> dict[str, Any]:
        if disclosed_quantity not in {"0", ""}:
            raise ValueError("disclosed_quantity is not supported by OpenAlgo Rust SDK v1.0.5")
        del disclosed_quantity  # OpenAlgo Rust SDK v1.0.5 does not expose this optional field.
        return self._decode(
            await self._client.place_order(
                strategy,
                symbol,
                action,
                exchange,
                pricetype,
                product,
                quantity,
                price,
                trigger_price,
            ),
        )

    async def modify_order(
        self,
        *,
        strategy: str,
        orderid: str,
        symbol: str,
        action: str,
        exchange: str,
        pricetype: str,
        product: str,
        quantity: str,
        price: str = "0",
        trigger_price: str = "0",
        disclosed_quantity: str = "0",
    ) -> dict[str, Any]:
        del trigger_price, disclosed_quantity
        return self._decode(
            await self._client.modify_order(
                orderid,
                strategy,
                symbol,
                action,
                exchange,
                pricetype,
                product,
                quantity,
                price,
            ),
        )

    async def cancel_order(self, *, strategy: str, orderid: str) -> dict[str, Any]:
        return self._decode(await self._client.cancel_order(orderid, strategy))

    async def cancel_all_order(self, *, strategy: str) -> dict[str, Any]:
        return self._decode(await self._client.cancel_all_order(strategy))

    async def order_status(self, *, strategy: str, orderid: str) -> dict[str, Any]:
        return self._decode(await self._client.order_status(orderid, strategy))

    def _decode(self, raw: str) -> dict[str, Any]:
        data = json.loads(raw)
        status = str(data.get("status", "")).lower()
        if status in {"error", "failure", "failed"}:
            message = data.get("message") or data.get("error") or data
            raise OpenAlgoHttpError(f"OpenAlgo request failed: {message}")
        return data
