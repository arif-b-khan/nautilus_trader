# Mixed Python + Rust Debugging Guide

This guide shows how to debug both Python and Rust code simultaneously from a Jupyter notebook in VS Code, with **automatic kernel PID detection** (no manual process selection needed).

## Prerequisites (One-time setup)

### 1. Install VS Code Extensions

Required extensions:
- **Rust Analyzer** (`rust-lang.rust-analyzer`)
- **CodeLLDB** (`vadimcn.vscode-lldb`)
- **Python** (`ms-python.python`)
- **Jupyter** (`ms-toolsai.jupyter`)

Install via VS Code Extensions marketplace or command line:
```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension ms-python.python
code --install-extension ms-toolsai.jupyter
```

### 2. Build with Debug Symbols

**Important:** You must build with PyO3 debug symbols for Rust breakpoints to work:

```bash
cd /Users/arifkhan/github/nautilus_trader
make build-debug-pyo3
```

This takes ~30 minutes on first build. You only need to rebuild when:
- Rust source code changes
- You switch between debug/release modes
- You pull new code that updates Rust crates

## Quick Start: Mixed Debugging

### Step 1: Open the Example Notebook

Open `examples/other/debugging/debug_mixed_jupyter.ipynb` in VS Code.

### Step 2: Run Setup Cell (with idempotent debugpy)

In the notebook's first cell, run:

```python
from nautilus_trader.test_kit.debug_helpers import setup_debugging

# Call with enable_python_debugging=False to avoid "already listening" error
# This still writes the launch.json with the current kernel PID
setup_debugging(enable_python_debugging=False, port=5678)
```

**What this does:**
- Detects the current Jupyter kernel's process ID (PID)
- Writes `.vscode/launch.json` with configurations that target this specific PID
- Avoids re-starting debugpy if it's already listening (common in VS Code Jupyter)

You'll see output like:
```
launch_json_path='/Users/arifkhan/github/nautilus_trader/.vscode/launch.json'
✓ VS Code configuration updated
Created 2 configurations and 1 compound configurations
1. In VS Code: Select 'Python + Rust Debugger (for Jupyter)' → Start Debugging (F5)
```

### Step 3: Set Breakpoints

- **Python:** Click in the gutter (left of line numbers) in any `.py` file
  - Example: `nautilus_trader/persistence/catalog/parquet.py` inside `_query_rust()`
  
- **Rust:** Click in the gutter in any `.rs` file
  - Example: `crates/persistence/src/backend/session.rs` inside `get_query_result()`

### Step 4: Start Mixed Debugging

1. Open the **Run and Debug** panel (Cmd+Shift+D or View → Run and Debug)
2. Select **"Python + Rust Debugger (for Jupyter)"** from the dropdown
3. Press **F5** (or click the green play button)

You'll see two debuggers attach:
- Python debugger connects to `localhost:5678`
- Rust debugger (LLDB) attaches to the kernel PID

### Step 5: Execute Notebook Cells

Run cells that call into Rust code (like the parquet catalog example). The debugger will pause at your breakpoints in both Python and Rust.

## Available Configurations

After running `setup_debugging()`, these configs appear in `.vscode/launch.json`:

1. **Python + Rust Debugger (for Jupyter)** ← Use this for mixed debugging
   - Attaches Python debugger to port 5678
   - Attaches Rust LLDB to the kernel PID (auto-detected)

2. **Python Debugger (for Jupyter)**
   - Python-only debugging (faster if you don't need Rust breakpoints)

3. **Rust Debugger (for Jupyter debugging)**
   - Rust-only debugging with LLDB

## Troubleshooting

### Error: "debugpy.listen() has already been called"

**Solution:** Use `enable_python_debugging=False` when calling `setup_debugging()`:

```python
setup_debugging(enable_python_debugging=False, port=5678)
```

This is normal in VS Code Jupyter where debugpy auto-starts. The helper will still write the launch.json with the correct PID.

### Rust Breakpoints Not Hitting

**Check these:**

1. **Did you build with debug symbols?**
   ```bash
   make build-debug-pyo3
   ```

2. **Is CodeLLDB extension installed?**
   - Check Extensions panel → search "CodeLLDB"
   - Reload VS Code after installing

3. **Did you run `setup_debugging()` in the notebook?**
   - This ensures launch.json has the current kernel PID

4. **Are you using the compound config?**
   - Select "Python + Rust Debugger (for Jupyter)" from the dropdown

5. **Is the notebook importing your workspace build?**
   Run this in the same notebook kernel and confirm the path points inside your workspace (not site-packages):

   ```python
   import nautilus_trader.core.nautilus_pyo3 as _pyo3
   print(_pyo3.__file__)
   ```

   Expected: `/Users/.../nautilus_trader/nautilus_trader/core/nautilus_pyo3.cpython-<py>-darwin.so`

   If it points to a global install (e.g. `.venv/site-packages`), rebuild and ensure the notebook kernel uses this workspace's env.

### LLDB Config Shows Schema Errors

If you see "type 'lldb' is not recognized" warnings in launch.json:

- Install the **CodeLLDB** extension (`vadimcn.vscode-lldb`)
- Reload VS Code
- Warnings will disappear

### Wrong Kernel PID

If you restart the Jupyter kernel or use a different notebook:

- **Re-run `setup_debugging(enable_python_debugging=False)` in the new kernel**
- This updates launch.json with the new PID

## Daily Workflow

1. Open Jupyter notebook in VS Code
2. Run `setup_debugging(enable_python_debugging=False)` once per kernel session
3. Set breakpoints in Python and Rust files
4. Start "Python + Rust Debugger (for Jupyter)" (F5)
5. Execute cells and debug

## Manual PID Selection (Alternative Method)

If you don't want to use `setup_debugging()`, you can manually select the kernel PID:

1. Use the launch config: **"Rust: Attach (LLDB -> Pick Python PID)"**
2. When prompted, search for your Python process (look for `.venv` path)
3. Select it from the list

This works but requires manual selection each time. The `setup_debugging()` approach is more convenient.

## Reference

- Full testing guide: `docs/developer_guide/testing.md`
- Debug helper source: `nautilus_trader/test_kit/debug_helpers.py`
- Example notebook: `examples/other/debugging/debug_mixed_jupyter.ipynb`
- PyO3 debugging docs: [https://pyo3.rs/v0.25.1/debugging.html](https://pyo3.rs/v0.25.1/debugging.html)

## Tips

- **Python-only debugging:** Use "Python Debugger (for Jupyter)" config (faster)
- **Rust-only debugging:** Use "Rust Debugger (for Jupyter debugging)" config
- **Background processes:** If debugging long-running code, use `justMyCode: false` in launch.json to step into libraries
- **Performance:** Debug builds are slower; switch back to release for production: `make install`
