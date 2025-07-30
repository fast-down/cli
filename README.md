# fast-down

![Latest commit (branch)](https://img.shields.io/github/last-commit/share121/fast-down/main)
[![Test](https://github.com/share121/fast-down/workflows/Test/badge.svg)](https://github.com/share121/fast-down/actions)
[![Latest version](https://img.shields.io/crates/v/fast-down.svg)](https://crates.io/crates/fast-down)
[![Documentation](https://docs.rs/fast-down/badge.svg)](https://docs.rs/fast-down)
![License](https://img.shields.io/crates/l/fast-down.svg)

`fast-down` **Fastest** concurrent downloader!

Languages: **en** [中文简体](./README_zhCN.md)

![CLI Interface](/docs/cli_en.png)

**[Official Website (Simplified Chinese)](https://fast.s121.top/)**

## Features

1. **⚡️ Fastest Download**  
   We created [fast-steal](https://github.com/share121/fast-steal) With optimized Work Stealing, **1.43 x faster** than
   NDM.
2. **🔄 File consistency**  
   Switching Wi-Fi, Turn Off Wi-Fi, Switch proxies. **We guarantee the consistency**.
3. **⛓️‍💥 Resuming Downloads**  
   You can **interrupt** at any time, and **resume downloading** after.
4. **⛓️‍💥 Incremental Downloads**  
   1000 more lines server logs? Don't worry, we **only download new lines**.
5. **💰 Free and open-source**  
   The code stays free and open-source. Thanks to [share121](https://github.com/share121), [Cyan](https://github.com/CyanChanges) and other fast-down contributors.
6. **💻 Cross platform**
   <table>
        <thead>
            <tr>
                <th>Arch</th>
                <th>Windows</th>
                <th>Linux</th>
                <th>Mac OS</th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>64 bit</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-64bit.zip">Download</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-linux-64bit.zip">Download</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-macos-64bit.zip">Download</a>
                </td>
            </tr>
            <tr>
                <td>32 bit</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-32bit.zip">Download</a>
                </td>
                <td>
                    ❌ Not Supported
                </td>
                <td>
                    ❌ Not Supported
                </td>
            </tr>
            <tr>
                <td>ARM 64</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-arm64.zip">Download</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-linux-arm64.zip">Download</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-macos-arm64.zip">Download</a>
                </td>
            </tr>
        </tbody>
    </table>

## Usage

```bash
> fast --help
超级快的下载器命令行界面

Usage: fast [OPTIONS] <URL>

Arguments:
  <URL>  要下载的URL

Options:
  -f, --allow-overwrite
          强制覆盖已有文件
      --no-allow-overwrite
          不强制覆盖已有文件
  -c, --continue
          断点续传
      --no-continue
          不断点续传
  -d, --dir <SAVE_FOLDER>
          保存目录
  -t, --threads <THREADS>
          下载线程数
  -o, --out <FILE_NAME>
          自定义文件名
  -p, --all-proxy <PROXY>
          代理地址 (格式: http://proxy:port 或 socks5://proxy:port)
  -H, --header <Key: Value>
          自定义请求头 (可多次使用)
      --write-buffer-size <WRITE_BUFFER_SIZE>
          写入缓冲区大小 (单位: B)
      --progress-width <PROGRESS_WIDTH>
          进度条显示宽度
      --retry-gap <RETRY_GAP>
          重试间隔 (单位: ms)
      --repaint-gap <REPAINT_GAP>
          进度条重绘间隔 (单位: ms)
      --browser
          模拟浏览器行为
      --no-browser
          不模拟浏览器行为
  -y, --yes
          全部确认
      --no-yes
          不全部确认
      --no
          全部拒绝
      --no-no
          不全部拒绝
  -v, --verbose
          详细输出
      --no-verbose
          不详细输出
  -h, --help
          Print help
```
