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

import os
from pathlib import Path


def _read_secret_file(path: Path) -> str:
    return path.read_text(encoding="utf-8").strip()


def _get_key_from_file_env(key: str) -> str | None:
    file_key = f"{key}_FILE"
    if file_key not in os.environ:
        return None

    file_path = os.environ[file_key]
    if not file_path:
        raise RuntimeError(f"Environment variable '{file_key}' is set but empty")

    try:
        return _read_secret_file(Path(file_path))
    except OSError as e:
        raise RuntimeError(
            f"Unable to read secret file for '{key}' from '{file_path}'",
        ) from e


def _get_key_from_dotenv(key: str) -> str | None:
    dotenv_path = Path(os.environ.get("NAUTILUS_DOTENV_FILE") or ".env")
    if not dotenv_path.is_file():
        return None

    try:
        with dotenv_path.open(encoding="utf-8") as f:
            for raw_line in f:
                line = raw_line.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue

                k, v = line.split("=", 1)
                if k.strip() != key:
                    continue

                value = v.strip()
                if value and ((value[0] == value[-1]) and value[0] in {'"', "'"}):
                    value = value[1:-1]
                return value
    except OSError as e:
        raise RuntimeError(f"Unable to read dotenv file '{dotenv_path}'") from e

    return None


def _get_key_from_secrets_dir(key: str) -> str | None:
    secrets_dir = Path(os.environ.get("NAUTILUS_SECRETS_DIR") or "/run/secrets")
    if not secrets_dir.is_dir():
        return None

    candidates = (
        secrets_dir / key,
        secrets_dir / key.lower(),
    )
    for path in candidates:
        if not path.is_file():
            continue
        try:
            return _read_secret_file(path)
        except OSError as e:
            raise RuntimeError(f"Unable to read secret file '{path}'") from e

    return None


def get_env_key(key: str) -> str:
    if key in os.environ:
        return os.environ[key]

    for resolver in (_get_key_from_file_env, _get_key_from_dotenv, _get_key_from_secrets_dir):
        value = resolver(key)
        if value is not None:
            return value

    raise RuntimeError(f"Environment variable '{key}' not set")


def get_env_key_or(key: str, default: str) -> str:
    try:
        return get_env_key(key)
    except RuntimeError:
        return default
