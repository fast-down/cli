# fast-down 快下

![Latest commit (branch)](https://img.shields.io/github/last-commit/share121/fast-down/main)
[![Test](https://github.com/share121/fast-down/workflows/Test/badge.svg)](https://github.com/share121/fast-down/actions)
[![Latest version](https://img.shields.io/crates/v/fast-down.svg)](https://crates.io/crates/fast-down)
[![Documentation](https://docs.rs/fast-down/badge.svg)](https://docs.rs/fast-down)
![License](https://img.shields.io/crates/l/fast-down.svg)

`fast-down` **全网最快**多线程下载库

语言: **中文简体** [en](./README.md)

![CLI 界面](/docs/cli_zhCN.png)

**[访问官网](https://fast.s121.top/)**

## 优势

1. **⚡️ 极速下载**  
   自研 [fast-steal](https://github.com/share121/fast-steal) 任务窃取算法，实测下载速度是 NDM 的 **2.43 倍**
2. **🔄 超强重试**  
   下载时，切换 WiFi、关闭 WiFi、切换代理，都能保证**文件内容正确**
3. **⛓️‍💥 断点续传**  
   下到一半**随时暂停**，之后还能**继续传输**
4. **⛓️‍💥 增量续传**  
   服务器日志今天下载完成，明天又多了 1000 行，增量续传功能实现**只传输新增的 1000 行**
5. **💰 开源免费**  
   所有代码全部公开，由 [share121](https://github.com/share121)、[Cyan](https://github.com/CyanChanges) 与其他贡献者一起维护
6. **💻 跨平台**
   <table>
        <thead>
            <tr>
                <th>架构</th>
                <th>Windows</th>
                <th>Linux</th>
                <th>Mac OS</th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>64 位</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-64bit.zip">下载</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-linux-64bit.zip">下载</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-macos-64bit.zip">下载</a>
                </td>
            </tr>
            <tr>
                <td>32 位</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-32bit.zip">下载</a>
                </td>
                <td>
                    ❌不支持
                </td>
                <td>
                    ❌不支持
                </td>
            </tr>
            <tr>
                <td>ARM 64</td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-windows-arm64.zip">下载</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-linux-arm64.zip">下载</a>
                </td>
                <td>
                    <a target="_blank" href="https://github.com/share121/fast-down/releases/latest/download/fast-down-macos-arm64.zip">下载</a>
                </td>
            </tr>
        </tbody>
    </table>

## 使用方法

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
