# Uploader

Multi-backend file upload CLI.

## Build

```bash
go build -trimpath -ldflags '-s -w' -o uploader .
```

## Usage

```bash
uploader -b temp ./file.bin
uploader backends
uploader probe
uploader probe temp lit gof -timeout 20
uploader encrypt -k secret ./file
uploader decrypt -k secret -o out.bin ./file.encrypt
uploader decrypt -k secret -force ./file.encrypt
```
