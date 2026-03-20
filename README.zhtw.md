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

![CLI 介面](https://fd.s121.top/cli.png)

**[造訪官網](https://fd.s121.top/)**

## 特色功能

- **⚡️ 極速下載**  
  自研 [fast-steal](https://github.com/fast-down/fast-steal) 工作竊取演算法，實測下載速度為 NDM 的 **2.43 倍**
- **🔄 強效重試**  
  下載過程中，切換 WiFi、關閉 WiFi、切換代理，皆可確保**檔案內容正確無誤**
- **⛓️‍💥 斷點續傳**  
  下載至一半可**隨時暫停**，後續仍能**繼續傳輸**
- **⛓️‍💥 增量續傳**  
  伺服器日誌今日下載完成後，明日若新增 1000 行，透過增量續傳功能僅需**傳輸新增的 1000 行**
- **💰 開源免費**  
  所有程式碼完全公開，由 [share121](https://github.com/share121)、[Cyan](https://github.com/CyanChanges) 及其他貢獻者共同維護
- **💻 跨平台支援**

## 下載

| 架構    | Windows   | Linux     | Mac OS    |
| ------- | --------- | --------- | --------- |
| x86_64  | [下載][1] | [下載][4] | [下載][7] |
| x86     | [下載][2] | [下載][5] | ❌ 不支援 |
| aarch64 | [下載][3] | [下載][6] | [下載][8] |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/x86_64
[2]: https://fast-down-update.s121.top/cli/download/latest/windows/i686
[3]: https://fast-down-update.s121.top/cli/download/latest/windows/aarch64
[4]: https://fast-down-update.s121.top/cli/download/latest/linux/x86_64
[5]: https://fast-down-update.s121.top/cli/download/latest/linux/i686
[6]: https://fast-down-update.s121.top/cli/download/latest/linux/aarch64
[7]: https://fast-down-update.s121.top/cli/download/latest/macos/x86_64
[8]: https://fast-down-update.s121.top/cli/download/latest/macos/aarch64

## 使用方法

```test
> fd download -h
fast-down v2.7.3
下載檔案 (預設)

Usage: fd download [OPTIONS] <URL>

Arguments:
  <URL>  要下載的 URL

Options:
  -f, --force
          強制覆蓋現有檔案
      --no-resume
          禁止斷點續傳
  -d, --dir <SAVE_FOLDER>
          儲存目錄 [default: .]
  -t, --threads <THREADS>
          下載執行緒數 [default: 32]
  -o, --out <FILE_NAME>
          自訂檔案名稱
  -p, --proxy <PROXY>
          代理位址 (格式: http://proxy:port 或 socks5://proxy:port) 不填為使用系統代理，-p "" 為不使用代理
  -H, --header <Key: Value>
          自訂請求標頭 (可多次使用)
      --min-chunk-size <MIN_CHUNK_SIZE>
          最小分片大小 (單位: B) [default: 1048576]
      --write-buffer-size <WRITE_BUFFER_SIZE>
          寫入緩衝區大小 (單位: B) [default: 8388608]
      --write-queue-cap <WRITE_QUEUE_CAP>
          寫入通道長度 [default: 10240]
      --progress-width <PROGRESS_WIDTH>
          進度條顯示寬度
      --retry-gap <RETRY_GAP>
          重試間隔 (單位: ms) [default: 500]
      --repaint-gap <REPAINT_GAP>
          進度條重繪間隔 (單位: ms) [default: 200]
      --pull-timeout <PULL_TIMEOUT>
          拉取超時時間 (單位: ms) [default: 5000]
      --browser
          模擬瀏覽器行為
  -y, --yes
          全部確認
  -v, --verbose
          詳細輸出
      --accept-invalid-certs
          允許無效憑證
      --accept-invalid-hostnames
          允許無效主機名稱
  -i, --interface
          是否使用互動式介面選擇網卡
      --ip <網卡的 ip 位址>
          自訂網卡 (可多次使用)
      --max-speculative <MAX_SPECULATIVE>
          最大投機執行緒數 [default: 3]
      --write-method <WRITE_METHOD>
          寫入方法 (mmap 速度快, std 相容性好) [default: mmap] [possible values: mmap, std]
  -h, --help
          Print help
```
