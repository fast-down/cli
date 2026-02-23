<div align="center">
<h1>fast-down: 高效能多執行緒下載器</h1>

<h3>極速下載 · 強效重試 · 斷點續傳 · 增量續傳</h3>

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

**繁體中文** | [简体中文](README.zhcn.md) | [English](README.md)

</div>



![CLI 介面](/docs/cli.png)

**[造訪官網](https://fd.s121.top/)**

## 特色功能

* **⚡️ 極速下載**  
  自研 [fast-steal](https://github.com/fast-down/fast-steal) 工作竊取演算法，實測下載速度為 NDM 的 **2.43 倍**
* **🔄 強效重試**  
  下載過程中，切換 WiFi、關閉 WiFi、切換代理，皆可確保**檔案內容正確無誤**
* **⛓️‍💥 斷點續傳**  
  下載至一半可**隨時暫停**，後續仍能**繼續傳輸**
* **⛓️‍💥 增量續傳**  
  伺服器日誌今日下載完成後，明日若新增 1000 行，透過增量續傳功能僅需**傳輸新增的 1000 行**
* **💰 開源免費**  
  所有程式碼完全公開，由 [share121](https://github.com/share121)、[Cyan](https://github.com/CyanChanges) 及其他貢獻者共同維護
* **💻 跨平台支援**

## 下載

| 架構    | Windows | Linux   | macOS   |
|-------|---------|---------|---------|
| 64 元  | [下載][1] | [下載][2] | [下載][3] |
| 32 元  | [下載][4] | [下載][8] | ❌ 不支援   |
| Arm64 | [下載][5] | [下載][6] | [下載][7] |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/64bit

[2]: https://fast-down-update.s121.top/cli/download/latest/linux/64bit

[3]: https://fast-down-update.s121.top/cli/download/latest/macos/64bit

[4]: https://fast-down-update.s121.top/cli/download/latest/windows/32bit

[5]: https://fast-down-update.s121.top/cli/download/latest/windows/arm64

[6]: https://fast-down-update.s121.top/cli/download/latest/linux/arm64

[7]: https://fast-down-update.s121.top/cli/download/latest/macos/arm64

[8]: https://fast-down-update.s121.top/cli/download/latest/linux/32bit
