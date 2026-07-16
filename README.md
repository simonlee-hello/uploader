# Uploader

多后端文件上传 CLI。拷贝单个二进制到目标机，即可把文件或目录上传到临时网盘 / 文件托管，并拿到下载链接。

适合无头环境、脚本调用，以及授权渗透测试中「C2 不适合拉大文件」时的云端中转外带。

Inspired by / based on [Mikubill/transfer](https://github.com/Mikubill/transfer)（已基本停更），本仓库在其思路上做了渠道维护、无头部署与故障转移等改进。

## Features

- **多后端**：temp.sh、litterbox、gofile、文叔叔等；可指定后端，也可 `-auto` 失败自动换路
- **目录友好**：目录默认先 zip 再传；`-r` 可按文件逐个上传
- **无头友好**：`-q` 只把链接打到 stdout，方便脚本 / C2 回显
- **体积克制**：Go 版偏 stdlib，体积约数 MB 级，便于投递
- **可选加密**：上传前加密，下载后本地解密
- **探测**：`probe` 先测渠道可用性，再传真正重要的包

## Install / Build

```bash
# Go（默认）
make all
# 或
CGO_ENABLED=0 go build -trimpath -ldflags '-s -w' -o uploader .

# 产物目录：bin/linux、bin/windows、bin/osx、bin/freebsd
```

可选 Rust 实现（实验）：

```bash
make build-rust
# 或交叉编译
make build-rust-cross
```

## Quick start

```bash
# 指定后端上传
uploader -b temp ./file.bin

# 目录：先压缩再上传
uploader -b lit ./mydir

# 安静模式 + 自动择路（推荐脚本 / 服务器）
uploader -q -auto ./file.bin
uploader -q -auto ./mydir

# 先探测可用后端
uploader backends
uploader probe
uploader probe temp lit gof -timeout 20
```

成功后 stdout（或默认输出）会给出下载链接；再用自己的环境从云端取回即可。

## Config

```bash
# 默认后端（未传 -b 时）
export UPLOADER_BACKEND=lit          # Linux / macOS
set UPLOADER_BACKEND=lit             # Windows cmd

# 配置文件
# Linux:   ~/.config/uploader/config
# Windows: %APPDATA%\uploader\config
# 示例:
#   backend=lit
#   auto=true

# 代理（对所有后端生效）
export https_proxy=http://127.0.0.1:6152
export http_proxy=http://127.0.0.1:6152
```

脚本里不要使用 `-keep`（会等待回车）。

## More commands

```bash
uploader -b lit -r ./mydir           # 目录内逐文件上传（不 zip）
uploader encrypt -k secret ./file
uploader decrypt -k secret -o out.bin ./file.encrypt
```

## Backend status

| Status | Meaning |
|--------|---------|
| ok | 稳定，默认池可用 |
| flaky | 不稳定；需要 `-force` |
| down | 已禁用；需要 `-force` |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | 成功 |
| 1 | 上传 / 配置错误 |
| 2 | 参数错误 |

## Disclaimer

仅用于授权渗透测试、攻防演练与防御研究。请勿用于未授权系统。使用者自行承担合规与法律风险。

## Credits

- [Mikubill/transfer](https://github.com/Mikubill/transfer) — 原始多后端上传思路与实现基础

## License

[MIT](./LICENSE)
