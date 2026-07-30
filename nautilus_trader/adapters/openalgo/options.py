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

from collections.abc import Iterable
from dataclasses import dataclass
from enum import Enum

from nautilus_trader.model.enums import OptionKind
from nautilus_trader.model.instruments import OptionContract


class RelativeStrikeKind(Enum):
    """
    Relative option strike category.
    """

    ATM = "ATM"
    ITM = "ITM"
    OTM = "OTM"


@dataclass(frozen=True)
class OpenAlgoRelativeStrike:
    """
    Parsed OpenAlgo-style relative strike selector.
    """

    kind: RelativeStrikeKind
    level: int = 0

    @classmethod
    def parse(cls, value: str) -> OpenAlgoRelativeStrike:
        normalized = value.strip()
        if normalized.startswith("[") and normalized.endswith("]"):
            normalized = normalized[1:-1]
        normalized = normalized.upper()

        if normalized in {"ATM", "ATM0", "ATM-0"}:
            return cls(RelativeStrikeKind.ATM)

        for name, kind in (
            ("ITM", RelativeStrikeKind.ITM),
            ("OTM", RelativeStrikeKind.OTM),
        ):
            if normalized.startswith(name):
                raw_level = normalized[len(name) :].removeprefix("-")
                try:
                    level = int(raw_level)
                except ValueError as exc:
                    raise ValueError(f"Invalid relative strike selector: {value}") from exc
                if not 1 <= level <= 50:
                    raise ValueError(f"{name} level must be between 1 and 50")
                return cls(kind, level)

        raise ValueError(f"Invalid relative strike selector: {value}")

    @property
    def canonical(self) -> str:
        if self.kind is RelativeStrikeKind.ATM:
            return "ATM"
        return f"{self.kind.value}{self.level}"


def resolve_openalgo_option(
    instruments: Iterable[OptionContract],
    *,
    underlying_price: float,
    option_kind: OptionKind,
    strike_selector: str | OpenAlgoRelativeStrike,
    expiration_ns: int,
) -> OptionContract:
    """
    Resolve an OpenAlgo-style relative strike to an exact Nautilus option contract.

    This helper performs no I/O and is suitable for deterministic backtesting.
    Calling it again may return a different contract as ``underlying_price`` moves.
    The caller must retain the returned instrument ID after submitting an order.
    """
    selector = (
        OpenAlgoRelativeStrike.parse(strike_selector)
        if isinstance(strike_selector, str)
        else strike_selector
    )
    candidates = [
        instrument
        for instrument in instruments
        if isinstance(instrument, OptionContract)
        and instrument.option_kind == option_kind
        and instrument.expiration_ns == expiration_ns
    ]
    if not candidates:
        raise ValueError(
            f"No {option_kind.name} option contracts found for expiration {expiration_ns}",
        )

    by_strike = {instrument.strike_price.as_double(): instrument for instrument in candidates}
    strikes = sorted(by_strike)
    atm_index = min(
        range(len(strikes)),
        key=lambda index: (abs(strikes[index] - underlying_price), strikes[index]),
    )

    direction = 0
    if selector.kind is RelativeStrikeKind.OTM:
        direction = 1 if option_kind is OptionKind.CALL else -1
    elif selector.kind is RelativeStrikeKind.ITM:
        direction = -1 if option_kind is OptionKind.CALL else 1

    target_index = atm_index + (direction * selector.level)
    if not 0 <= target_index < len(strikes):
        raise ValueError(
            f"{selector.canonical} is unavailable for {option_kind.name} at "
            f"underlying price {underlying_price}",
        )
    return by_strike[strikes[target_index]]
