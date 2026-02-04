# fast-down 快下

![Latest commit (branch)](https://img.shields.io/github/last-commit/fast-down/cli/main)
[![Test](https://github.com/fast-down/cli/workflows/Test/badge.svg)](https://github.com/fast-down/cli/actions)
[![Latest version](https://img.shields.io/crates/v/fast-down-cli.svg)](https://crates.io/crates/fast-down-cli)
![License](https://img.shields.io/crates/l/fast-down-cli.svg)

`fast-down` **全网最快**多线程下载库

语言: **中文简体** [en](./README.md)

![CLI 界面](/docs/cli.png)

**[访问官网](https://fast.s121.top/)**

## 优势

1. **⚡️ 极速下载**  
   自研 [fast-steal](https://github.com/fast-down/fast-steal) 任务窃取算法，实测下载速度是 NDM 的 **2.43 倍**
2. **🔄 超强重试**  
   下载时，切换 WiFi、关闭 WiFi、切换代理，都能保证**文件内容正确**
3. **⛓️‍💥 断点续传**  
   下到一半**随时暂停**，之后还能**继续传输**
4. **⛓️‍💥 增量续传**  
   服务器日志今天下载完成，明天又多了 1000 行，增量续传功能实现**只传输新增的 1000 行**
5. **💰 开源免费**  
   所有代码全部公开，由 [share121](https://github.com/share121)、[Cyan](https://github.com/CyanChanges) 与其他贡献者一起维护
6. **💻 跨平台**

   | 架构    | Windows | Linux   | Mac OS  |
   |-------|---------|---------|---------|
   | 64 位  | [下载][1] | [下载][2] | [下载][3] |
   | 32 位  | [下载][4] | [下载][8] | ❌ 不支持   |
   | Arm64 | [下载][5] | [下载][6] | [下载][7] |

[1]: https://fast-down-update.s121.top/cli/download/latest/windows/64bit
[2]: https://fast-down-update.s121.top/cli/download/latest/linux/64bit
[3]: https://fast-down-update.s121.top/cli/download/latest/macos/64bit
[4]: https://fast-down-update.s121.top/cli/download/latest/windows/32bit
[5]: https://fast-down-update.s121.top/cli/download/latest/windows/arm64
[6]: https://fast-down-update.s121.top/cli/download/latest/linux/arm64
[7]: https://fast-down-update.s121.top/cli/download/latest/macos/arm64
[8]: https://fast-down-update.s121.top/cli/download/latest/linux/32bit

## 使用方法

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
