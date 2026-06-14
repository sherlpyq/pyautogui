# ⚡ PyAutoGUI-Rust & Driver Integration

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows-0078d7.svg?style=flat-square&logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/Language-Python%20%7C%20Rust-orange.svg?style=flat-square" alt="Language">
  <img src="https://img.shields.io/badge/Version-v1.0.0-blue.svg?style=flat-square" alt="Version">
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=flat-square" alt="License">
</p>

---

本项目是对经典 GUI 自动化库 **PyAutoGUI** 的全方位重构版。通过引入 **Rust 原生加速** 和 **Windows 内核驱动级底层接口**，彻底解决原版运行缓慢、易被检测、截图与找图效率低下的痛点。

---

## 🚀 核心特性与技术革新 (Key Features)

### 🛡️ 1. 内核驱动级仿真输入 (Kernel Driver Input)
原版使用 Windows API 的 `SendInput`，在高权限窗口或游戏内易被拦截或检测。  
**本作提升**：集成了底层内核驱动接口（通过 `\\.\MyDriver` 通信）。开启驱动模式后，操作将在内核态直接模拟物理硬件输入，可完美绕过各类应用层防作弊检测。

### ⚡ 2. 毫秒级 GDI 截图加速 (GDI Screenshot)
原版使用 Pillow 进行屏幕抓取，速度慢且占用大量 CPU。  
**本作提升**：在 Rust 扩展中直接调用原生 Windows GDI 进行屏幕内存截取。图像数据在 C 连续内存中直接读取，截图速度提升 5~10 倍。

### 🔍 3. 极速 Rust 金字塔找图 (Hierarchical Template Matching)
原版通过 Python 逐像素比对或依赖第三方大库，耗时长。  
**本作提升**：使用 Rust 实现了**分层金字塔 SAD 图像匹配算法**。自动对屏幕和大图进行下采样以快速收敛搜索范围，大图检索实现毫秒级响应。

### 🛑 4. 线程级底层 Failsafe 钩子 (Active Failsafe Hook)
原版仅在同步操作间隙检测鼠标位置，无法做到实时中断。  
**本作提升**：在 Rust 中通过独立线程注册 Windows 底层鼠标钩子 (`WH_MOUSE_LL`)。鼠标一触及屏幕四角立即触发原子状态中断，保护更加安全及时。

### ⏱️ 5. 高精度毫秒级定时器
原生配置 Windows 系统时钟分辨率为 1ms，确保 `sleep` 延时控制精准，鼠标平滑移动无卡顿。

---

## 📊 性能对比 (Performance Comparison)

| 功能模块 | 原始版本 (PyAutoGUI) | 本定制版 (Rust + Driver) | 提升效果 |
| :--- | :--- | :--- | :--- |
| **鼠标/键盘控制** | API级模拟 (易被屏蔽检测) | 内核驱动级硬件仿真 (防检测) | **安全与穿透力质的提升** |
| **屏幕截图** | Pillow (约 50-100ms) | GDI 内存映射 (约 5-10ms) | **快 10 倍** |
| **屏幕图像定位** | 纯 Python / OpenCV (耗时较长) | Rust 图像金字塔 SAD 匹配 | **毫秒级极速定位** |
| **安全中断 (Failsafe)** | 周期同步轮询 (有延迟) | Win32 底层鼠标钩子线程 (零延迟) | **更安全及时** |

---

## 📂 项目结构 (Project Structure)

```
pyautogui/
├── PyAutoGUI-0.9.54/          # 带有 Rust 原生加速的 PyAutoGUI 核心包
│   ├── src/                   # Rust 扩展源码 (编译为 _rust_core.pyd)
│   └── rust_installer/        # 驱动程序安装包与启动器
└── rust_driver_demo/          # Rust 驱动交互独立示例项目
```

---

## 🛠️ 安装指引 (Installation)

### 1. 安装 Wheel 分发包
直接从 Releases 页面下载编译好的 `.whl` 文件安装：
```bash
pip install pyautogui-0.9.54-cp37-abi3-win_amd64.whl
```

### 2. 驱动配置与启用
管理员权限下运行驱动安装工具：
1. 运行 `rust_installer.exe` 安装驱动。
2. 运行 `pyautogui_loader.exe` 挂载服务。
3. Python 中正常调用库即可体验超强内核加速：
```python
import pyautogui
```
