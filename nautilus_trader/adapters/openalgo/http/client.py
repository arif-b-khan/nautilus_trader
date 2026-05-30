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

import os
from typing import Any

import aiohttp


class OpenAlgoHttpError(RuntimeError):
    """
    Raised when the OpenAlgo HTTP API returns an error response.
    """


class OpenAlgoHttpClient:
    """
    Thin async client for OpenAlgo's local REST API.
    """

    def __init__(
        self,
        api_key: str | None = None,
        base_url: str = "http://127.0.0.1:5000",
        api_version: str = "v1",
        timeout_secs: int = 10,
        proxy_url: str | None = None,
        session: aiohttp.ClientSession | None = None,
    ) -> None:
        self._api_key = api_key or os.getenv("OPENALGO_API_KEY")
        if not self._api_key:
            raise ValueError("OpenAlgo API key not provided; set api_key or OPENALGO_API_KEY")

        self._base_url = base_url.rstrip("/")
        self._api_version = api_version.strip("/")
        self._timeout = aiohttp.ClientTimeout(total=timeout_secs)
        self._proxy_url = proxy_url
        self._session = session
        self._owns_session = session is None

    async def connect(self) -> None:
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession(timeout=self._timeout)

    async def close(self) -> None:
        if self._owns_session and self._session is not None and not self._session.closed:
            await self._session.close()

    async def ping(self) -> dict[str, Any]:
        return await self._post("ping")

    async def funds(self) -> dict[str, Any]:
        return await self._post("funds")

    async def orderbook(self) -> dict[str, Any]:
        return await self._post("orderbook")

    async def tradebook(self) -> dict[str, Any]:
        return await self._post("tradebook")

    async def positionbook(self) -> dict[str, Any]:
        return await self._post("positionbook")

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
        return await self._post(
            "placeorder",
            {
                "strategy": strategy,
                "symbol": symbol,
                "action": action,
                "exchange": exchange,
                "pricetype": pricetype,
                "product": product,
                "quantity": quantity,
                "price": price,
                "trigger_price": trigger_price,
                "disclosed_quantity": disclosed_quantity,
            },
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
        return await self._post(
            "modifyorder",
            {
                "strategy": strategy,
                "orderid": orderid,
                "symbol": symbol,
                "action": action,
                "exchange": exchange,
                "pricetype": pricetype,
                "product": product,
                "quantity": quantity,
                "price": price,
                "trigger_price": trigger_price,
                "disclosed_quantity": disclosed_quantity,
            },
        )

    async def cancel_order(self, *, strategy: str, orderid: str) -> dict[str, Any]:
        return await self._post("cancelorder", {"strategy": strategy, "orderid": orderid})

    async def cancel_all_order(self, *, strategy: str) -> dict[str, Any]:
        return await self._post("cancelallorder", {"strategy": strategy})

    async def order_status(self, *, strategy: str, orderid: str) -> dict[str, Any]:
        return await self._post("orderstatus", {"strategy": strategy, "orderid": orderid})

    async def _post(
        self,
        endpoint: str,
        payload: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        await self.connect()
        assert self._session is not None

        url = f"{self._base_url}/api/{self._api_version}/{endpoint}"
        body = {"apikey": self._api_key}
        if payload:
            body.update(payload)

        async with self._session.post(url, json=body, proxy=self._proxy_url) as resp:
            try:
                data = await resp.json(content_type=None)
            except Exception as e:
                text = await resp.text()
                raise OpenAlgoHttpError(
                    f"OpenAlgo {endpoint} returned non-JSON HTTP {resp.status}: {text}",
                ) from e

        status = str(data.get("status", "")).lower()
        if resp.status >= 400 or status in {"error", "failure", "failed"}:
            message = data.get("message") or data.get("error") or data
            raise OpenAlgoHttpError(f"OpenAlgo {endpoint} failed: {message}")

        return data
