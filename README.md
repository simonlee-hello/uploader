# Transfer

集合多个 API 的大文件传输工具

## support

文件上传范例

```bash
./transfer <backend> <your-file-path>

./transfer wet /home/user/file.bin
```

目前支持的文件传输服务：

|  Name  | Command | Site  | Limit    | alive            |
|  ----  | ----  | ----  |----------|------------------|
| Airportal | `arp` | https://airportal.cn/ | -        | 需要登录             |
| CatBox | `cat` | https://catbox.moe/ | 200MB    | checkin 20240814 |
| Fileio | `fio` | https://file.io/ | 2GB      | checkin 20240814 |
| GoFile | `gof` | https://gofile.io/ | no limit | checkin 20240814 |
| Wenshushu | `wss` | https://wenshushu.cn/ | 2GB      | checkin 20240814 |
| WeTransfer | `wet` | https://wetransfer.com/ | 2GB      | 已注释，不使用          |
| LitterBox | `lit` | https://litterbox.catbox.moe/ | 1GB      | checkin 20240814 |
| 1Fichier | `fic` | https://www.1fichier.com/ | 300GB    | checkin 20240814 |
| Null | `null` | https://0x0.st/ | 512M     | checkin 20240814 |
| Infura (ipfs) | `inf` | https://infura.io/ | 128M     | 需要登录             |
| DownloadGG | `gg` | https://download.gg/ | -        | checkin 20240815 |
| tmp.link|`tmp`|https://tmp.link/| 50G      | 需要登录             |

需要登录才能使用的服务：

|  Name   | Command | Site  | 
|  ----  | ----  |  ----  |  
|Airportal|`arp`|https://airportal.cn/|
| Lanzous | `lzs` | https://www.lanzous.com/ |
| CowTransfer | `cow` | https://www.cowtransfer.com/ | 
| tmp.link|`tmp`|https://tmp.link/|

已失效或不可用的服务：

|  Name   | Site  | 
|  ----  | ----  |  
| Vim-cn | https://img.vim-cn.com/ |
| WhiteCats | http://whitecats.dip.jp/ |

部分服务仅支持上传；部分服务需要使用 beta 版本。


[登陆上传相关说明](https://github.com/Mikubill/transfer#login)

## usage 

```text
Transfer is a very simple big file transfer tool.

Backend Support:
  airportal(arp), catbox(cat), cowtransfer(cow), fileio(fio),
  gofile(gof), lanzous(lzs), litterbox(lit), null(0x0), 
  wetransfer(wet), vimcn(vim)

Usage:
  transfer [flags]
  transfer [command]

Examples:
  # upload via wenshushu
  ./transfer wss <your-file>

  # download link
  ./transfer https://.../

Available Commands:
  decrypt     Decrypt a file
  encrypt     Encrypt a file
  hash        Hash a file
  help        Help about any command
  image       Upload a image to imageBed

Flags:
      --encrypt              encrypt stream when upload
      --encrypt-key string   specify the encrypt key
  -f, --force                attempt to download file regardless error
  -h, --help                 help for transfer
      --keep                 keep program active when process finish
      --no-progress          disable progress bar to reduce output
  -o, --output string        download to another file/folder (default ".")
  -p, --parallel int         set download task count (default 3)
      --silent               enable silent mode to mute output
  -t, --ticket string        set download ticket
      --verbose              enable verbose mode to debug
      --version              show version and exit

Use "transfer [command] --help" for more information about a command.
```

### upload & download

所有上传操作都建议指定一个 API，如不指定将使用默认 (fileio.Backend)。加上想要传输的文件/文件夹即可。

```text

Upload a file or folder.

Usage:
  transfer [flags] <files>

Aliases:
  upload, up

Flags:
      --encrypt              Encrypt stream when upload
      --encrypt-key string   Specify the encrypt key
  -h, --help                 help for upload

Global Flags:
      --no-progress          disable progress bar to reduce output
      --silent               enable silent mode to mute output
      --keep                 keep program active when process finish
      --version              show version and exit

Use "transfer upload [command] --help" for more information about a command.
```

Examples

```shell script
# upload
./transfer balabala.mp4

# upload
./transfer wss balabala.mp4

# upload folder
./transfer wet /path/
```

不同的 Backend 提供不同的选项，可以在帮助中查看关于该服务的相关信息。

```text
➜  ./transfer cow
cowTransfer - https://cowtransfer.com/

  Size Limit:             2G(Anonymous), ~100G(Login)
  Upload Service:         qiniu object storage, East China
  Download Service:       qiniu cdn, Global

Usage:
  transfer cow [flags]

Aliases:
  cow, cow, cowtransfer

Flags:
      --block int         Upload block size (default 262144)
  -c, --cookie string     Your user cookie (optional)
      --hash              Check hash after block upload
  -h, --help              help for cow
  -p, --parallel int      Set the number of upload threads (default 2)
      --password string   Set password
  -s, --single            Upload multi files in a single link
  -t, --timeout int       Request retry/timeout limit in second (default 10)

Global Flags:
      --encrypt              encrypt stream when upload
      --encrypt-key string   specify the encrypt key
      --keep                 keep program active when process finish
      --no-progress          disable progress bar to reduce output
      --silent               enable silent mode to mute output
      --verbose              enable verbose mode to debug
      --version              show version and exit
```

下载操作会自动识别支持的链接，不需要指定服务名称。

```shell script
# download file
./transfer https://.../
```

试验性功能：`--encrypt`选项可以在上传时将文件加密，下载时需要配合`--decrypt`选项才能正确下载文件。（当然也可以先下载后再解密）加密方式为 AES-CBC，默认会自动生成一个密码，也可以通过`--encrypt-key`指定一个。

```shell script 
# encrypt stream when upload
➜ ./transfer wss --encrypt transfer
Warning: crypto mode is enabled.
Note: Crypto mode still in beta and abnormalities may occur, do not over-rely on this function.
Key is not set or incorrect: Setting it to 94d0500605b372245dc77f95fbc20010
...

# encrypt with key
➜ ./transfer wss --encrypt --encrypt-key=123 transfer
Warning: crypto mode is enabled.
Note: Crypto mode still in beta and abnormalities may occur, do not over-rely on this function.
Encrypt using key: 123
...

# decrypt stream when download
➜ ./transfer --encrypt --encrypt-key=123 https://....
Warning: crypto mode is enabled.
Note: Crypto mode is not compatible with multi thread download mode, setting parallel to 1.
...
```

### login 

部分 backend 支持登陆环境下上传，使用时只需要提供对应的 cookie 即可。

CowTransfer

```shell script
# login to upload
./transfer cow --cookie="remember-mev2=...;" -a "<cow-auth-token>" file
```

AirPortal

```shell script
# login to upload
./transfer arp -t <your-token> -u <your-username> file
```

TmpLink 
```shell script
# login to upload
./transfer tmp -t <your-token> file
```

Lanzous

蓝奏云可以只使用 `phpdisk_info` 项作为 cookie 上传文件，但可能无法进行文件管理（如删除等）。如需要上传到指定目录或进行文件管理操作需要在 cookie 中指定 `folder_id_c` 的值，如：

```shell script
# login to upload (without path)
./transfer lzs --cookie='phpdisk_info=...' file

# login to upload (with path)
./transfer lzs --cookie='phpdisk_info=...; folder_id_c=...;' file
```

### encrypt & decrypt

和前面 upload 使用的是同样的加密，只是在本地进行。也可以使用前面下载的加密后文件在此解密。可以通过不同参数指定密钥和输出文件名

关于加密的说明：目前只能选择 AES-CBC 的加密方式，分块大小策略为 min(1m, fileSize)

```shell script 
# encrypt
transfer encrypt your-file

# encrypt using specified key
transfer encrypt -k abc your-file

# decrypt using specified key
transfer decrypt -k abc your-file

# specify path
transfer encrypt -o output your-file
```

### hash 

hash 功能使用 sha1, crc32, md5, sha256 对文件进行校验，可以用来检验文件一致性。

```shell script 
➜  ./transfer hash main.go
size: 68
path: /../transfer/main.go

crc32: a51da8f5
md5: aa091bb918ab85b1dc44cb771b1663d1
sha1: a8e25d41330c545da8bcbeade9aebdb1b4a13ab7
sha256: ab4dd3cdd79b5e2a88fcb3fcd45dfcffc935c913adfa888f3fb50b324638e958
```
