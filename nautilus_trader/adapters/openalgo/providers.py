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

from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.model.identifiers import InstrumentId


class OpenAlgoInstrumentProvider(InstrumentProvider):
    """
    Minimal OpenAlgo instrument provider.

    OpenAlgo routes to broker symbols supplied by the user, so this provider does not
    currently discover instruments. Strategies should load/cache Nautilus instruments
    separately, or use simple IDs such as ``RELIANCE.NSE``.
    """

    def __init__(self, config: InstrumentProviderConfig | None = None) -> None:
        super().__init__(config=config or InstrumentProviderConfig())

    async def load_all_async(self, filters: dict | None = None) -> None:
        self._log.warning("OpenAlgo instrument discovery is not available; loaded 0 instruments")

    async def load_ids_async(
        self,
        instrument_ids: list[InstrumentId],
        filters: dict | None = None,
    ) -> None:
        self._log.warning(
            "OpenAlgo instrument discovery is not available; "
            f"unable to load {len(instrument_ids)} instruments",
        )

    async def load_async(
        self,
        instrument_id: InstrumentId,
        filters: dict | None = None,
    ) -> None:
        self._log.warning(
            f"OpenAlgo instrument discovery is not available; unable to load {instrument_id}",
        )
