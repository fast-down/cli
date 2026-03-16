<div align="center">
<h1>fast-down: High-Performance Multi-Threaded Downloader</h1>

<h3>Lightning Speed · Robust Retry · Resumable Download · Incremental Resumption</h3>

<p>
   <img src="https://img.shields.io/badge/Build with-Rust-DEA584?style=flat&logo=rust&logoColor=white" alt="Rust">
   <img src="https://img.shields.io/badge/Arch-x86__64%2C%20x86%2C%20ARM64-blue" alt="Hardware">
   <img src="https://img.shields.io/badge/OS-Windows%2C%20macOS%2C%20Linux-orange" alt="Hardware">
   <br>
   <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
   <img src="https://img.shields.io/github/last-commit/fast-down/cli/main" alt="Last commit">
   <img src="https://github.com/fast-down/cli/workflows/Test/badge.svg" alt="Test">
   <img src="https://img.shields.io/crates/v/fast-down-cli.svg" alt="Latest version">
   <br>
   <a href="https://fd.s121.top/"><img src="https://img.shields.io/badge/Website-fd.s121.top-blue?style=flat&logo=google-chrome&logoColor=white" alt="Website"></a>
<a href="https://dc.vacu.top/"><img src="https://img.shields.io/badge/Discord-Online-5865F2.svg?logo=discord&logoColor=white" alt="Discord"></a>
</p>

**English** | [简体中文](README.zhcn.md) | [繁體中文](README.zhtw.md)

</div>

![CLI Interface](https://fd.s121.top/cli.png)

**[Visit Official Website](https://fd.s121.top/)**

## Features

- **⚡️ Lightning-Fast Download**  
  Built-in [fast-steal](https://github.com/fast-down/fast-steal) work-stealing algorithm, with a measured download speed
  **2.43x that of NDM**.
- **🔄 Robust Retry**  
  During download, switching WiFi, disconnecting WiFi, or changing proxies will still guarantee **correct file content
  **.
- **⛓️‍💥 Resumable Download**  
  Pause **at any time** halfway, and **resume the transfer** later.
- **⛓️‍💥 Incremental Resumption**  
  If a server log is fully downloaded today and 1000 new lines are added tomorrow, this feature only transfers the **new
  1000 lines**.
- **💰 Open-Source & Free**  
  Full source code is public, maintained
  by [share121](https://github.com/share121), [Cyan](https://github.com/CyanChanges), and other contributors.
- **💻 Cross-Platform**

## Download

| Architecture | Windows       | Linux         | Mac OS           |
| ------------ | ------------- | ------------- | ---------------- |
| x86-64       | [Download][1] | [Download][4] | ❌ Not supported |
| x86          | [Download][2] | [Download][5] | ❌ Not supported |
| aarch64      | [Download][3] | [Download][6] | [Download][7]    |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/x86_64
[2]: https://fast-down-update.s121.top/cli/download/latest/windows/i686
[3]: https://fast-down-update.s121.top/cli/download/latest/windows/aarch64
[4]: https://fast-down-update.s121.top/cli/download/latest/linux/x86_64
[5]: https://fast-down-update.s121.top/cli/download/latest/linux/i686
[6]: https://fast-down-update.s121.top/cli/download/latest/linux/aarch64
[7]: https://fast-down-update.s121.top/cli/download/latest/macos/aarch64
