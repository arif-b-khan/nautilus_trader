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

import pytest
from aiohttp import web

from nautilus_trader.adapters.openalgo.http.client import OpenAlgoHttpClient
from nautilus_trader.adapters.openalgo.http.client import OpenAlgoHttpError


@pytest.mark.asyncio
async def test_place_order_posts_openalgo_sdk_payload(aiohttp_server):
    received = {}

    async def handler(request):
        received.update(await request.json())
        return web.json_response({"status": "success", "orderid": "250408000989443"})

    app = web.Application()
    app.router.add_post("/api/v1/placeorder", handler)
    server = await aiohttp_server(app)

    client = OpenAlgoHttpClient(api_key="KEY", base_url=str(server.make_url("")))
    try:
        response = await client.place_order(
            strategy="NautilusTrader",
            symbol="RELIANCE",
            action="BUY",
            exchange="NSE",
            pricetype="MARKET",
            product="MIS",
            quantity="1",
        )
    finally:
        await client.close()

    assert response == {"status": "success", "orderid": "250408000989443"}
    assert received == {
        "apikey": "KEY",
        "strategy": "NautilusTrader",
        "symbol": "RELIANCE",
        "action": "BUY",
        "exchange": "NSE",
        "pricetype": "MARKET",
        "product": "MIS",
        "quantity": "1",
    }


@pytest.mark.asyncio
async def test_error_status_raises(aiohttp_server):
    async def handler(request):
        return web.json_response({"status": "error", "message": "bad order"})

    app = web.Application()
    app.router.add_post("/api/v1/cancelorder", handler)
    server = await aiohttp_server(app)

    client = OpenAlgoHttpClient(api_key="KEY", base_url=str(server.make_url("")))
    try:
        with pytest.raises(OpenAlgoHttpError, match="bad order"):
            await client.cancel_order(strategy="NautilusTrader", orderid="1")
    finally:
        await client.close()
