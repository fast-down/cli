# fast-down 快下

![GitHub last commit (branch)](https://img.shields.io/github/last-commit/share121/fast-down/main)
[![Rust](https://github.com/share121/fast-down/workflows/Test/badge.svg)](https://github.com/share121/fast-down/actions)
[![Latest version](https://img.shields.io/crates/v/fast-down.svg)](https://crates.io/crates/fast-down)
[![Documentation](https://docs.rs/fast-down/badge.svg)](https://docs.rs/fast-down)
![License](https://img.shields.io/crates/l/fast-down.svg)

`fast-down` 是一个特别快的多线程库，支持超细颗粒度的任务窃取。

## 优势

1. 无锁
2. 安全的 Rust 代码
3. 跨平台，Windows、Linux、Mac OS 都支持

```powershell
> ./fast-down.exe -h
超级快的下载器

Usage: fast-down.exe [OPTIONS] <URL>

Arguments:
  <URL>  要下载的URL

Options:
  -f, --force                      强制覆盖已有文件
  -d, --save-folder <SAVE_FOLDER>  保存目录 [default: .]
  -t, --threads <THREADS>          下载线程数 [default: 32]
  -n, --file-name <FILE_NAME>      自定义文件名
  -x, --proxy <PROXY>              代理地址 (格式: http://proxy:port 或 socks5://proxy:port)
  -H, --headers <HEADER>           自定义请求头 (格式: "Key: Value"，可多次使用)
  -h, --help                       Print help
  -V, --version                    Print version
```
