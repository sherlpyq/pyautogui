# PyAutoGUI Rust Driver Integration

本项目是对原始 PyAutoGUI 的深度定制与性能升级版本。通过引入 Rust 底层原生加速与 Windows 内核驱动接口，在执行效率、图像识别速度及防检测能力上实现了质的提升。

## 改动与提升说明 (Improvements & Differences)

### 1. 内核级驱动模拟输入 (Kernel Driver Emulation)
* **原有机制**：原版使用 Windows API 的 `SendInput` 进行模拟。极易被游戏防作弊系统或高权限窗口屏蔽。
* **提升改动**：集成了底层内核驱动接口（通过 `\\.\MyDriver` 通信）。开启驱动模式后，按键与鼠标事件将直接通过驱动在内核态发送，实现完美的物理硬件级模拟，完美绕过应用层防作弊检测。

### 2. 高性能 GDI 截图加速 (GDI Screenshot Acceleration)
* **原有机制**：原版使用 Python 库 `Pillow` 进行屏幕抓取，存在较高的 CPU 占用和内存拷贝开销。
* **提升改动**：在 Rust 扩展中直接调用 Windows GDI (`BitBlt` 与 `GetDIBits`) 进行原生屏幕截取。画面数据在 C 语言连续内存中高速缓存，截图速度相比原版提升数倍。

### 3. Rust 实现的高速图像检索 (Rust Native Template Matching)
* **原有机制**：原版使用 OpenCV 或 Pillow 逐像素比对（Python 实现），效率极低。
* **提升改动**：在 Rust 层实现了**分层金字塔 SAD 匹配算法 (Hierarchical SAD Matching)**：
  * 对屏幕和大图进行下采样（金字塔加速），快速锁定候选区域。
  * 在候选区域进行局部高精度 SAD 比对。
  * 检索速度达到毫秒级，且大大降低了 CPU 耗时。

### 4. 线程级低延时 Failsafe 安全钩子 (Low-level Failsafe Hook)
* **原有机制**：原版在每次操作间隙同步获取鼠标位置判断是否在角落，若操作无间隙则无法及时触发中断。
* **提升改动**：在 Rust 中使用独立线程注册了 Windows 低级鼠标钩子 (`WH_MOUSE_LL`)。一旦鼠标移动到屏幕四角，立即通过原子操作触发中断，响应速度更快、更安全。

### 5. 高精度毫秒级定时器与 DPI 适配
* 自动启用 Windows 系统的高精度定时器（设置时钟分辨率为 1ms），提供极其精准的 `sleep` 和平滑鼠标轨迹。
* 自动开启 Windows `Per-Monitor DPI Aware V2` 适配，解决多分辨率/多缩放屏幕下的定位偏差问题。

---

## 项目结构 (Project Structure)

* `PyAutoGUI-0.9.54/` - 带有 Rust 核心加速的 PyAutoGUI 模块包。
  * `src/` - Rust 原生扩展源码。
  * `rust_installer/` - 驱动加载器与配置安装工具。
* `rust_driver_demo/` - Rust 驱动控制独立演示程序。

## 安装与使用 (Installation & Usage)

### 安装打包的 Wheel 文件
从 Releases 中下载编译好的 `.whl` 包直接安装：

```bash
pip install pyautogui-0.9.54-cp37-abi3-win_amd64.whl
```

### 驱动控制配置
* 使用管理员权限运行 `rust_installer.exe` 安装驱动。
* 运行 `pyautogui_loader.exe` 挂载驱动接口。
* 在 Python 代码中开启驱动：

```python
import pyautogui
```
