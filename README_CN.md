<p align="center">
  中文 | <a href="./README.md">English</a>
</p>
<h1 align="center"><img src="./picture/lanscanner-readme.png" alt="LANScanner logo" width="55" align="absmiddle" /> LANScanner</h1>

**你的终极局域网探索与 SSH 管理利器。**

欢迎来到 LANScanner！这是一款功能强大、速度极快且设计优雅的网络实用工具。专为对性能和直观用户体验有极高要求的开发者、系统管理员以及 IT 爱好者打造。

你是否厌倦了在各种不同的命令行工具之间切换，仅仅为了查找 IP 地址、检查开放端口或是管理你的 SSH 连接？LANScanner 完美解决了这个问题，它提供了一个全能的图形化界面，让你的本地网络管理尽在掌控之中。

### 为什么选择 LANScanner？

- 🚀 **极速扫描：** 由 Rust 和异步网络库 (`tokio`) 强力驱动，LANScanner 能够在几秒钟内飞速扫描你的子网并发现活跃设备。
- 🎨 **美观直观的 UI：** 基于 `Iced` 框架构建，提供现代、响应式且流畅的图形界面，让网络管理不再是繁杂的任务，而是一种享受。
- 🔐 **无缝 SSH 与凭据管理：** 直接在应用内管理你的 SSH 凭据，并一键连接到你的服务器。再也不用到处翻找配置文件或忘记密码了。
- 🐳 **Docker 与服务检测：** 智能识别 Docker 容器和正在运行的服务，让你深入了解网络中实际运行的内容。
- 🛠 **开箱即用：** 零配置需求。只需启动应用，选择你的网络接口，点击扫描。就是这么简单！

---

## 此处下载

- 下载总览：https://github.com/ChickmagnetL/LANScanner/releases/latest
- Windows 下载：https://github.com/ChickmagnetL/LANScanner/releases/latest/download/LANScanner-windows-x86_64.zip
- MacOS 下载：[后续更新...]
- Linux 下载：[后续更新...]

---

## 🚀 使用说明

1. **启动应用。**

2. **选择网络：** 在应用界面左侧的扫描网络卡片中，点击下拉框，选择你想要扫描的网络接口。

   <p align="center"><img src="./picture/ChooseNetwork.jpg" alt="Choose Network" /></p>

3. **开始扫描：** 点击扫描按钮。见证设备、开放端口和服务在列表中实时弹出的过程。

   <p align="center"><img src="./picture/Scanning.jpg" alt="Scanning" /></p>

4. **连接设备：**

   - 在左侧 SSH 登录凭证栏填写用户名和密码，或者使用内置的凭据管理器保存 SSH 用户名和密码/密钥。然后点击下方的检测 SSH 凭证。
   - 点击通过验证的设备，以查看可供使用的快速连接应用选项。
   - 只需点击一下右侧栏的图标，即可快速打开对应的应用，并连接此 IP 设备。

   <p align="center"><img src="./picture/connect.jpg" alt="Connect" /></p>

---

## 🏗 架构说明

LANScanner 经过精心设计，采用模块化的 Workspace 架构，以确保代码的可维护性、清晰的关注点分离以及未来的可扩展性。项目主要分为以下几个专门的 crate：

- **`crates/app`**: 应用程序的主要入口和协调者。它将业务逻辑与用户界面结合在一起，处理主要的应用状态、消息路由和 shell 集成。
- **`crates/core`**: 应用程序的核心心脏。包含网络扫描、SSH 协议处理 (`russh`)、Docker 检测和凭据管理的所有繁重工作。这部分纯粹由逻辑驱动，与 UI 无关。
- **`crates/platform`**: 处理特定操作系统的集成和系统级操作。包括查找网络接口、启动终端模拟器或外部应用，以及窗口和进程管理。
- **`crates/ui`**: 视觉展示层。完全使用 `Iced` 构建，包含可重用的组件（widgets）、主题、设备列表、扫描卡片和模态对话框。

---

## 🛠 构建说明

为了极大地简化编译过程，LANScanner 摒弃了繁琐的手动命令，采用一键构建应用的脚本。

1. **克隆仓库并进入目录：**

   ```bash
   git clone https://github.com/ChickmagnetL/LANScanner.git
   cd LANScanner
   ```

2. **使用构建脚本：**
   所有自动化构建脚本都统一存放在 `tools/build/` 目录下。Windows 环境使用 `windows.ps1`，Linux 环境使用 `linux.sh`。

*注：`tools/build/linux.sh` 现在会产出 `release/linux/LANScanner-x86_64.AppImage` 作为 Linux 发布物。它需要在 Linux 内运行，不能当作原生 Windows `.exe` 直接启动。macOS 的一键构建脚本后续再补充。*

首次在 Linux 上构建时，请确保系统里有 `python3` 以及 `curl` 或 `wget`。脚本会用它们提取应用自带的 `ico` 图标并按需下载本地打包工具。

---

## 🗺 未来计划

近期的项目路线图主要包括：

- [ ] 🍎 **适配 macOS 平台**
- [ ] 🐧 **适配 Linux 平台**

------

## 致谢

[Linux.Do](https://linux.do/) 社区
