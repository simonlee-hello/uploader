# Uploader

Multi-backend file upload CLI (stdlib-only, ~6MB). Copy a single binary to Windows/Linux and upload files or directories.

## Build

```bash
make all
# or one target:
CGO_ENABLED=0 go build -trimpath -ldflags '-s -w' -o uploader .
```

Binaries: `bin/linux/*`, `bin/windows/*`, `bin/osx/*` (amd64, arm64, x86 where applicable).

## Headless / server deploy

```bash
# Quiet: stdout = links only; stderr = errors only
uploader -q -auto ./file.bin
uploader -q -auto ./mydir              # zip dir, upload

# Default backend (no -b): temp, or override:
export UPLOADER_BACKEND=lit            # Linux/macOS
set UPLOADER_BACKEND=lit                 # Windows cmd

# Config file (~/.config/uploader/config on Linux, %APPDATA%\uploader\config on Windows):
# backend=lit
# auto=true

# Proxy (all backends):
export https_proxy=http://127.0.0.1:6152
export http_proxy=http://127.0.0.1:6152

# Do NOT use -keep in scripts (waits for Enter).
```

## Usage

```bash
uploader -b temp ./file.bin
uploader -b lit ./mydir              # zip directory, then upload
uploader -b lit -r ./mydir           # upload each file under directory
uploader -q -auto ./file             # quiet + failover
uploader backends
uploader probe
uploader probe temp lit gof -timeout 20
uploader encrypt -k secret ./file
uploader decrypt -k secret -o out.bin ./file.encrypt
```

## Backend status

| Status | Meaning |
|--------|---------|
| ok | Stable, default pool |
| flaky | Unreliable; needs `-force` |
| down | Disabled; needs `-force` |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Upload/config error |
| 2 | Bad flags |
