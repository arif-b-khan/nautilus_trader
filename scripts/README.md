# Scripts directory

This directory contains assorted helper scripts used by NautilusTrader’s
developer tooling and CI pipeline.

---

## `setup-dev.sh` / `setup-dev.ps1` - one-time local development setup

These scripts install or verify the local prerequisites needed to build the
project from source, sync Python dependencies, build NautilusTrader in debug
mode, and verify that the local package imports.

Linux and macOS:

```bash
./scripts/setup-dev.sh
```

Windows PowerShell:

```powershell
.\scripts\setup-dev.ps1
```

What the setup performs:

- installs `uv` if it is missing
- installs `rustup` if it is missing
- installs the Rust version from `rust-toolchain.toml`
- updates the `stable` Rust toolchain used by `build.py`
- installs/checks platform build tools
  - Linux: `clang`, `make`, and C/C++ build packages via the detected package manager
  - macOS: Apple command line tools
  - Windows: Visual Studio Build Tools with C++ tools via `winget`
- runs `uv sync --all-groups --all-extras --inexact --no-install-package nautilus_trader`
- runs `BUILD_MODE=debug uv run --no-sync build.py`
- verifies `import nautilus_trader`

The first build can take several minutes because it compiles Rust crates and
Cython extensions. Re-running the script is safe; existing tools and cached
build artifacts are reused where possible.

After setup, you can run an existing backtest from the repository root:

```bash
printf '\n' | uv run --no-sync python examples/backtest/fx_ema_cross_audusd_ticks.py
```

---

Only one of the remaining scripts (`curate-dataset.sh`) needs a brief
explanation because it is meant to be executed manually when curating
test-fixture datasets.

---

## `curate-dataset.sh` – package an external dataset for the test-data bucket

`curate-dataset.sh` automates the small but repetitive tasks required when we
bring a third-party file into the NautilusTrader *test-data* bucket:

- download the raw file from its original URL (with retries)
- create a versioned directory (`v1/<slug>/`)
- copy the file into that directory
- write a `LICENSE.txt` file holding the SPDX identifier or licence URL
- compute size and SHA-256 checksum and store them in `metadata.json`

The result is a self-contained directory ready to upload one-for-one to the
S3 bucket (or to commit into the repository if the data size is small).

### Usage

```bash
scripts/curate-dataset.sh <slug> <filename> <download-url> <licence>
```

- **`slug`** – sub-directory name (e.g. `fi2010_all`)
- **`filename`** – the basename you want inside the directory (e.g. `Fi2010.zip`)
- **`download-url`** – original public URL of the file
- **`licence`** – short ID or full URL (e.g. `CC-BY-SA-4.0`)

Example – curate the full FI-2010 limit-order-book dataset (all 10 trading
days) from a Dropbox mirror:

```bash
scripts/curate-dataset.sh fi2010_all Fi2010.zip \
  "https://www.dropbox.com/s/6ywf3td7zdrp1n5/Fi2010.zip?dl=1" \
  CC-BY-SA-4.0
```

After the script finishes you will have the following structure ready to
commit or upload:

```
v1/fi2010_all/
 ├── Fi2010.zip          # ≈230 MB, contains day_1 … day_10
 ├── LICENSE.txt         # CC-BY-SA-4.0
 └── metadata.json       # size, sha256, provenance
```

You can now reference `v1/fi2010_all/Fi2010.zip` from tests or example code,
and downstream tooling can verify the checksum.

### Notes

- The script uses `curl -L --fail --retry 3`, so transient network hiccups are
  handled automatically.
- Re-running the script with the same arguments simply overwrites the existing
  files – useful when the upstream file is updated and you want to bump the
  checksum.
- Only basic validation is performed; ensure that the licence you specify
  indeed permits redistribution.

---

For details on the other helper scripts, run them with `-h` or read the
inline comments; they are mostly invoked from CI and rarely need manual
execution.
