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

from typing import Final

from nautilus_trader.model.identifiers import ClientId


OPENALGO: Final[str] = "OPENALGO"
OPENALGO_CLIENT_ID: Final[ClientId] = ClientId(OPENALGO)
OPENALGO_DEFAULT_BASE_URL: Final[str] = "http://127.0.0.1:5000"
OPENALGO_DEFAULT_WS_URL: Final[str] = "ws://127.0.0.1:8765"
OPENALGO_DEFAULT_API_VERSION: Final[str] = "v1"
OPENALGO_DEFAULT_VENUE: Final[str] = "NSE"
OPENALGO_DEFAULT_API_KEY: Final[str] = ""
