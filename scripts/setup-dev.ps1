$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..")
$RustToolchainFile = Join-Path $RepoRoot "rust-toolchain.toml"

function Write-Step {
    param([string]$Message)
    Write-Host "> $Message" -ForegroundColor Cyan
}

function Add-SetupPath {
    $paths = @(
        "$env:USERPROFILE\.cargo\bin",
        "$env:USERPROFILE\.local\bin",
        "$env:ProgramFiles\LLVM\bin",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer"
    )

    foreach ($path in $paths) {
        if ((Test-Path $path) -and (($env:Path -split ";") -notcontains $path)) {
            $env:Path = "$path;$env:Path"
        }
    }
}

function Get-RustVersion {
    $content = Get-Content $RustToolchainFile -Raw
    if ($content -notmatch 'version\s*=\s*"([^"]+)"') {
        throw "Could not read Rust version from $RustToolchainFile"
    }
    return $Matches[1]
}

function Test-Command {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Install-Uv {
    if (Test-Command "uv") {
        Write-Step "uv already installed: $(uv --version)"
        return
    }

    Write-Step "Installing uv..."
    Invoke-RestMethod https://astral.sh/uv/install.ps1 | Invoke-Expression
    Add-SetupPath

    if (-not (Test-Command "uv")) {
        throw "uv was installed, but is not on PATH. Add %USERPROFILE%\.local\bin to PATH and rerun."
    }

    Write-Step "Installed $(uv --version)"
}

function Install-Rust {
    if (-not (Test-Command "rustup")) {
        Write-Step "Installing rustup..."
        $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
        & $rustupInit -y --profile minimal
        Add-SetupPath
    }

    if (-not (Test-Command "rustup")) {
        throw "rustup is required"
    }

    $rustVersion = Get-RustVersion
    Write-Step "Installing Rust toolchain $rustVersion..."
    rustup toolchain install $rustVersion

    # build.py defaults RUSTUP_TOOLCHAIN to "stable" and only accepts stable/nightly.
    Write-Step "Updating Rust stable toolchain..."
    rustup update stable
    $env:RUSTUP_TOOLCHAIN = if ($env:RUSTUP_TOOLCHAIN) { $env:RUSTUP_TOOLCHAIN } else { "stable" }
    Write-Step "Using $(rustup run $env:RUSTUP_TOOLCHAIN rustc --version)"
}

function Install-WindowsBuildTools {
    $hasCl = Test-Command "cl"
    $hasVsWhere = Test-Command "vswhere"
    if (-not $hasVsWhere -and (Test-Path "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe")) {
        $hasVsWhere = $true
    }

    if ($hasCl -or $hasVsWhere) {
        Write-Step "Visual Studio build tools appear to be installed"
        return
    }

    if (-not (Test-Command "winget")) {
        throw "Visual Studio Build Tools are required. Install them manually, or install winget and rerun this script."
    }

    Write-Step "Installing Visual Studio 2022 Build Tools with C++ tools..."
    winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --silent --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621"
    Add-SetupPath
}

function Sync-Dependencies {
    Write-Step "Syncing Python dependencies into the project virtual environment..."
    uv sync --all-groups --all-extras --inexact --no-install-package nautilus_trader
}

function Build-Project {
    Write-Step "Building NautilusTrader locally in debug mode..."
    $env:BUILD_MODE = if ($env:BUILD_MODE) { $env:BUILD_MODE } else { "debug" }
    $env:CARGO_TARGET_DIR = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
    $env:RUSTUP_TOOLCHAIN = if ($env:RUSTUP_TOOLCHAIN) { $env:RUSTUP_TOOLCHAIN } else { "stable" }
    uv run --no-sync build.py
}

function Verify-Install {
    Write-Step "Verifying local import..."
    uv run --no-sync python -c "import nautilus_trader; from nautilus_trader.core import nautilus_pyo3; print(f'nautilus_trader {nautilus_trader.__version__}'); print(f'pyo3 module {nautilus_pyo3.__name__}')"
}

Set-Location $RepoRoot
Add-SetupPath
Install-WindowsBuildTools
Install-Uv
Install-Rust
Sync-Dependencies
Build-Project
Verify-Install
Write-Step "Setup complete. You can now run examples with: uv run --no-sync python examples/backtest/fx_ema_cross_audusd_ticks.py"
