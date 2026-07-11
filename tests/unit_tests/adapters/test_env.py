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

from nautilus_trader.adapters.env import get_env_key
from nautilus_trader.adapters.env import get_env_key_or


def test_get_env_key_returns_environment_value(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setenv("OPENALGO_API_KEY", "from-env")

    # Act
    result = get_env_key("OPENALGO_API_KEY")

    # Assert
    assert result == "from-env"


def test_get_env_key_returns_file_value(monkeypatch: pytest.MonkeyPatch, tmp_path) -> None:
    # Arrange
    secret_file = tmp_path / "openalgo.key"
    secret_file.write_text("from-file\n", encoding="utf-8")
    monkeypatch.delenv("OPENALGO_API_KEY", raising=False)
    monkeypatch.setenv("OPENALGO_API_KEY_FILE", str(secret_file))

    # Act
    result = get_env_key("OPENALGO_API_KEY")

    # Assert
    assert result == "from-file"


def test_get_env_key_returns_dotenv_value(monkeypatch: pytest.MonkeyPatch, tmp_path) -> None:
    # Arrange
    monkeypatch.delenv("OPENALGO_API_KEY", raising=False)
    monkeypatch.delenv("OPENALGO_API_KEY_FILE", raising=False)
    dotenv = tmp_path / ".env"
    dotenv.write_text("OPENALGO_API_KEY=from-dotenv\n", encoding="utf-8")
    monkeypatch.chdir(tmp_path)

    # Act
    result = get_env_key("OPENALGO_API_KEY")

    # Assert
    assert result == "from-dotenv"


def test_get_env_key_returns_secrets_dir_value(monkeypatch: pytest.MonkeyPatch, tmp_path) -> None:
    # Arrange
    monkeypatch.delenv("OPENALGO_API_KEY", raising=False)
    monkeypatch.delenv("OPENALGO_API_KEY_FILE", raising=False)
    monkeypatch.setenv("NAUTILUS_DOTENV_FILE", str(tmp_path / "missing.env"))
    monkeypatch.setenv("NAUTILUS_SECRETS_DIR", str(tmp_path))
    (tmp_path / "openalgo_api_key").write_text("from-secret-dir\n", encoding="utf-8")

    # Act
    result = get_env_key("OPENALGO_API_KEY")

    # Assert
    assert result == "from-secret-dir"


def test_get_env_key_or_returns_default_when_unset(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.delenv("MISSING_KEY", raising=False)

    # Act
    result = get_env_key_or("MISSING_KEY", "default-value")

    # Assert
    assert result == "default-value"
