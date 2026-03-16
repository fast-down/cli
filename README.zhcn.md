<div align="center">
<h1>fast-down: 高性能多线程下载器</h1>

<h3>极速下载 · 超强重试 · 断点续传 · 增量续传</h3>

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

**简体中文** | [繁體中文](README.zhtw.md) | [English](README.md)

</div>

![CLI 界面](https://fd.s121.top/cli.png)

**[访问官网](https://fd.s121.top/)**

## 特性

- **⚡️ 极速下载**  
  自研 [fast-steal](https://github.com/fast-down/fast-steal) 任务窃取算法，实测下载速度是 NDM 的 **2.43 倍**
- **🔄 超强重试**  
  下载时，切换 WiFi、关闭 WiFi、切换代理，都能保证**文件内容正确**
- **⛓️‍💥 断点续传**  
  下到一半**随时暂停**，之后还能**继续传输**
- **⛓️‍💥 增量续传**  
  服务器日志今天下载完成，明天又多了 1000 行，增量续传功能实现**只传输新增的 1000 行**
- **💰 开源免费**  
  所有代码全部公开，由 [share121](https://github.com/share121)、[Cyan](https://github.com/CyanChanges) 与其他贡献者一起维护
- **💻 跨平台**

## 下载


| 架构     | Windows       | Linux         | Mac OS           |
| ------- | ------------- | ------------- | ---------------- |
| x86-64  | [Download][1] | [Download][4] | ❌ Not supported |
| x86     | [Download][2] | [Download][5] | ❌ Not supported |
| aarch64 | [Download][3] | [Download][6] | [Download][7]    |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/x86_64
[2]: https://fast-down-update.s121.top/cli/download/latest/windows/i686
[3]: https://fast-down-update.s121.top/cli/download/latest/windows/aarch64
[4]: https://fast-down-update.s121.top/cli/download/latest/linux/x86_64
[5]: https://fast-down-update.s121.top/cli/download/latest/linux/i686
[6]: https://fast-down-update.s121.top/cli/download/latest/linux/aarch64
[7]: https://fast-down-update.s121.top/cli/download/latest/macos/aarch64

## 使用方法

```text
> fd download -h
fast-down v2.7.3
下载文件 (默认)

Usage: fd download [OPTIONS] <URL>

Arguments:
  <URL>  要下载的URL

Options:
  -f, --force
          强制覆盖已有文件
      --no-resume
          禁止断点续传
  -d, --dir <SAVE_FOLDER>
          保存目录 [default: .]
  -t, --threads <THREADS>
          下载线程数 [default: 32]
  -o, --out <FILE_NAME>
          自定义文件名
  -p, --proxy <PROXY>
          代理地址 (格式: http://proxy:port 或 socks5://proxy:port) 不填为使用系统代理，-p "" 为不使用代理
  -H, --header <Key: Value>
          自定义请求头 (可多次使用)
      --min-chunk-size <MIN_CHUNK_SIZE>
          最小分片大小 (单位: B) [default: 1048576]
      --write-buffer-size <WRITE_BUFFER_SIZE>
          写入缓冲区大小 (单位: B) [default: 8388608]
      --write-queue-cap <WRITE_QUEUE_CAP>
          写入通道长度 [default: 10240]
      --progress-width <PROGRESS_WIDTH>
          进度条显示宽度
      --retry-gap <RETRY_GAP>
          重试间隔 (单位: ms) [default: 500]
      --repaint-gap <REPAINT_GAP>
          进度条重绘间隔 (单位: ms) [default: 200]
      --pull-timeout <PULL_TIMEOUT>
          拉取超时时间 (单位: ms) [default: 5000]
      --browser
          模拟浏览器行为
  -y, --yes
          全部确认
  -v, --verbose
          详细输出
      --accept-invalid-certs
          允许无效证书
      --accept-invalid-hostnames
          允许无效主机名
  -i, --interface
          是否使用交互式界面选择网卡
      --ip <网卡的 ip 地址>
          自定义网卡 (可多次使用)
      --max-speculative <MAX_SPECULATIVE>
          最大投机线程数 [default: 3]
      --write-method <WRITE_METHOD>
          写入方法 (mmap 速度快, std 兼容性好) [default: mmap] [possible values: mmap, std]
  -h, --help
          Print help
```
