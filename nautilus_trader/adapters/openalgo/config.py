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

from nautilus_trader.adapters.openalgo.constants import OPENALGO_DEFAULT_API_VERSION
from nautilus_trader.adapters.openalgo.constants import OPENALGO_DEFAULT_BASE_URL
from nautilus_trader.adapters.openalgo.constants import OPENALGO_DEFAULT_VENUE
from nautilus_trader.adapters.openalgo.constants import OPENALGO_DEFAULT_WS_URL
from nautilus_trader.common.config import PositiveInt
from nautilus_trader.config import LiveExecClientConfig


class OpenAlgoExecClientConfig(LiveExecClientConfig, frozen=True):
    """
    Configuration for ``OpenAlgoExecutionClient`` instances.

    Parameters
    ----------
    api_key : str, optional
        The OpenAlgo app API key. If ``None`` then ``OPENALGO_API_KEY`` is used.
    base_url_http : str, default ``http://127.0.0.1:5000``
        The OpenAlgo HTTP host.
    base_url_ws : str, default ``ws://127.0.0.1:8765``
        The OpenAlgo WebSocket host. Reserved for market-data streaming support.
    api_version : str, default ``v1``
        The OpenAlgo REST API version.
    venue : str, default ``NSE``
        The Nautilus venue/exchange code for this client instance.
    product : str, default ``MIS``
        The OpenAlgo product sent on order requests unless overridden later.
    strategy : str, default ``NautilusTrader``
        The OpenAlgo strategy tag sent on order requests.
    account_id : str, optional
        The account ID to publish into Nautilus. Defaults to ``OPENALGO-<venue>``.
    base_currency : str, default ``INR``
        The account base currency.
    http_timeout_secs : PositiveInt, default 10
        The timeout (seconds) for HTTP requests.
    http_proxy_url : str, optional
        Optional HTTP proxy URL.

    """

    api_key: str | None = OPENALGO_DEFAULT_API_KEY
    base_url_http: str = OPENALGO_DEFAULT_BASE_URL
    base_url_ws: str = OPENALGO_DEFAULT_WS_URL
    api_version: str = OPENALGO_DEFAULT_API_VERSION
    venue: str = OPENALGO_DEFAULT_VENUE
    product: str = "MIS"
    strategy: str = "NautilusTrader"
    account_id: str | None = None
    base_currency: str = "INR"
    http_timeout_secs: PositiveInt = 10
    http_proxy_url: str | None = None
