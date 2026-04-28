<p align="center">
  <a href="./README_CN.md">中文</a> | English
</p>
<h1 align="center"><img src="./picture/lanscanner-readme.png" alt="LANScanner logo" width="55" align="absmiddle" /> LANScanner</h1>

**Discover local network devices, verify SSH connections, and launch VS Code, Terminal, VNC, or RustDesk with a single click.**

LANScanner is more than just a LAN scanner. It streamlines the process of finding devices, confirming connectivity, and launching them with a familiar tool into a seamless desktop workflow, eliminating the need to manually check IPs, test ports, repeatedly enter credentials, and switch between applications.

If you frequently switch between development boards, minicomputers, Raspberry Pis, NAS, or servers, LANScanner helps you quickly identify which machines are online and which can be accessed via SSH. Then, choose a suitable remote access tool for a one-click connection.

### Why Choose LANScanner?

- 🚀 **One-Click Remote Access:** After scanning for devices, you can directly launch VS Code, Terminal, VNC, or RustDesk within your application without manually copying IP addresses.

- 🔐**Fast Connection After Verification:** After successful SSH credential verification, connections can be initiated directly from within the application, allowing passwordless login to the aforementioned remote software, reducing repetitive input and switching between credentials.

- ⚡**Lightning-Fast Experience:** Built on Rust, both interface feedback and LAN scanning speed remain sleek and smooth.

- 🗂**SSH Credential Management:** Frequently used usernames and passwords can be centrally stored and managed, making connections to multiple devices more efficient.

- 🖥**Easy to Use:** The intuitive layout design makes it easy to understand how to operate the application from the first glance, as if it were innate.

- 🐳**Docker Container Connection:** The application can detect the Docker environment of the target device and supports one-click remote login to Docker containers (via the Vscode Dev Containers extension).

---

## Downloads

- Release overview: https://github.com/ChickmagnetL/LANScanner/releases/latest
- Windows download: https://github.com/ChickmagnetL/LANScanner/releases/latest/download/LANScanner-windows-x86_64.zip
- macOS download: https://github.com/ChickmagnetL/LANScanner/releases/latest/download/LANScanner-macos.dmg
- Linux download: https://github.com/ChickmagnetL/LANScanner/releases/latest/download/LANScanner-linux-x86_64.AppImage

---

## 🚀 Usage

1. **Launch the application.**

2. **Choose a network:** In the "Scan Networks" tab on the left side of the application interface, click the drop-down box and select the network interface you want to scan.

   <p align="center"><img src="./picture/ChooseNetwork_EN.jpg" alt="Choose Network" /></p>

3. **Start scanning:** Click the scan button and watch devices, open ports, and detected services appear in real time.

   <p align="center"><img src="./picture/Scanning_EN.jpg" alt="Scanning" /></p>

4. **Connect to a device:**

   - Enter the username and password in the SSH credential panel on the left, or save SSH usernames and passwords/keys with the built-in credential manager. Then click the SSH credential verification button below.
   - Click a verified device to view the available quick-launch connection applications.
   - Click an icon in the right-side panel to open the corresponding application and connect to that device IP immediately.

   <p align="center"><img src="./picture/connect_EN.jpg" alt="Connect" /></p>

---

## 🏗 Architecture

LANScanner is organized as a modular Cargo workspace so the codebase stays maintainable, responsibilities stay clearly separated, and future expansion remains straightforward. The project is mainly split into the following crates:

- **`crates/app`**: The main application entry point and coordinator. It combines business logic with the user interface and handles top-level app state, message routing, and shell integration.
- **`crates/core`**: The core engine of the application. It contains the heavy lifting for network scanning, SSH protocol handling with `russh`, Docker detection, and credential management. This layer is logic-focused and independent from the UI.
- **`crates/platform`**: Handles OS-specific integrations and system-level operations, including network interface discovery, launching terminal emulators or external applications, and window/process management.
- **`crates/ui`**: The presentation layer. It is built entirely with `Iced` and contains reusable widgets, themes, device lists, scan cards, and modal dialogs.

---

## 🛠 Build

Users who need to modify the application can use the one-click application build script provided in the project after modifying the code, which greatly simplifies the compilation process.

1. **Clone the repository and enter the project directory:**

   ```bash
   git clone https://github.com/ChickmagnetL/LANScanner.git
   cd LANScanner
   ```

2. **Run the build script:**
   All automated build scripts are grouped under `tools/build/`. Use `windows.ps1` on Windows, `macos.sh` on macOS, and `linux.sh` in a Linux environment such as native Linux.

---

## 🗺 Roadmap

The near-term roadmap currently includes:

- [x] **macOS support **
- [x] **Linux support 🐧**

## Acknowledgments

[Linux.Do](https://linux.do/) Community
