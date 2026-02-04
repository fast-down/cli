# fast-down

![Latest commit (branch)](https://img.shields.io/github/last-commit/fast-down/cli/main)
[![Test](https://github.com/fast-down/cli/workflows/Test/badge.svg)](https://github.com/fast-down/cli/actions)
[![Latest version](https://img.shields.io/crates/v/fast-down-cli.svg)](https://crates.io/crates/fast-down-cli)
![License](https://img.shields.io/crates/l/fast-down-cli.svg)

`fast-down` **Fastest** concurrent downloader!

Languages: **en** [中文简体](./README_zhCN.md)

![CLI Interface](/docs/cli.png)

**[Official Website (Simplified Chinese)](https://fd.s121.top/)**

## Features

1. **⚡️ Fastest Download**  
   We created [fast-steal](https://github.com/fast-down/fast-steal) With optimized Work Stealing, **1.43 x faster** than NDM.
2. **🔄 File consistency**  
   Switching Wi-Fi, Turn Off Wi-Fi, Switch proxies. **We guarantee the consistency**.
3. **⛓️‍💥 Resuming Downloads**  
   You can **interrupt** at any time, and **resume downloading** after.
4. **⛓️‍💥 Incremental Downloads**  
   1000 more lines server logs? Don't worry, we **only download new lines**.
5. **💰 Free and open-source**  
   The code stays free and open-source. Thanks to [share121](https://github.com/share121), [Cyan](https://github.com/CyanChanges) and other fast-down contributors.
6. **💻 Cross platform**

   | Arch   | Windows       | Linux           | Mac OS          |
   |--------|---------------|-----------------|-----------------|
   | 64 bit | [Download][1] | [Download][2]   | [Download][3]   |
   | 32 bit | [Download][4] | [Download][8] | ❌ Not Supported |
   | Arm64  | [Download][5] | [Download][6]    | [Download][7]    |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/64bit
[2]: https://fast-down-update.s121.top/cli/download/latest/linux/64bit
[3]: https://fast-down-update.s121.top/cli/download/latest/macos/64bit
[4]: https://fast-down-update.s121.top/cli/download/latest/windows/32bit
[5]: https://fast-down-update.s121.top/cli/download/latest/windows/arm64
[6]: https://fast-down-update.s121.top/cli/download/latest/linux/arm64
[7]: https://fast-down-update.s121.top/cli/download/latest/macos/arm64
[8]: https://fast-down-update.s121.top/cli/download/latest/linux/32bit

## Usage

```bash
> fd download -h
fast-down v2.6.0
下载文件 (默认)

Usage: fd.exe download [OPTIONS] <URL>

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
          代理地址 (格式: http://proxy:port 或 socks5://proxy:port) [default: ]
  -H, --header <Key: Value>
          自定义请求头 (可多次使用)
      --write-buffer-size <WRITE_BUFFER_SIZE>
          写入缓冲区大小 (单位: B) [default: 8388608]
      --write-queue-cap <WRITE_QUEUE_CAP>
          写入通道长度 [default: 10240]
      --progress-width <PROGRESS_WIDTH>
          进度条显示宽度
      --retry-gap <RETRY_GAP>
          重试间隔 (单位: ms) [default: 500]
      --repaint-gap <REPAINT_GAP>
          进度条重绘间隔 (单位: ms) [default: 100]
      --browser
          模拟浏览器行为
  -y, --yes
          全部确认
  -v, --verbose
          详细输出
      --multiplexing
          开启多路复用 (不推荐)
      --accept-invalid-certs
          允许无效证书
      --accept-invalid-hostnames
          允许无效主机名
  -h, --help
          Print help
```
