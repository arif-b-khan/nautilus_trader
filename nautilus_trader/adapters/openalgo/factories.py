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
from functools import lru_cache

from nautilus_trader.adapters.openalgo.config import OpenAlgoExecClientConfig
from nautilus_trader.adapters.openalgo.execution import OpenAlgoExecutionClient
from nautilus_trader.adapters.openalgo.http.client import OpenAlgoHttpClient
from nautilus_trader.adapters.openalgo.providers import OpenAlgoInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.live.factories import LiveExecClientFactory


@lru_cache(1)
def get_cached_openalgo_http_client(
    api_key: str | None = None,
    base_url: str = "http://127.0.0.1:5000",
    api_version: str = "v1",
    ws_url: str = "ws://127.0.0.1:8765",
    timeout_secs: int = 10,
    proxy_url: str | None = None,
) -> OpenAlgoHttpClient:
    """
    Cache and return an OpenAlgo HTTP client.
    """
    return OpenAlgoHttpClient(
        api_key=api_key,
        base_url=base_url,
        api_version=api_version,
        ws_url=ws_url,
        timeout_secs=timeout_secs,
        proxy_url=proxy_url,
    )


@lru_cache(1)
def get_cached_openalgo_instrument_provider(
    config: InstrumentProviderConfig | None = None,
) -> OpenAlgoInstrumentProvider:
    """
    Cache and return an OpenAlgo instrument provider.
    """
    return OpenAlgoInstrumentProvider(config=config)


class OpenAlgoLiveExecClientFactory(LiveExecClientFactory):
    """
    Provides an OpenAlgo live execution client factory.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: OpenAlgoExecClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> OpenAlgoExecutionClient:
        client = get_cached_openalgo_http_client(
            api_key=config.api_key,
            base_url=config.base_url_http,
            api_version=config.api_version,
            ws_url=config.base_url_ws,
            timeout_secs=config.http_timeout_secs,
            proxy_url=config.http_proxy_url,
        )
        provider = get_cached_openalgo_instrument_provider(config=config.instrument_provider)
        return OpenAlgoExecutionClient(
            loop=loop,
            client=client,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=provider,
            config=config,
            name=name,
        )
