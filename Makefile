# 动态编译
BUILD_ENV := CGO_ENABLED=0
LDFLAGS-Linux := -ldflags '-s -w -extldflags "-static"' -gcflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}" -asmflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}"
LDFLAGS-Win := -ldflags '-s -w -extldflags "-static"' -gcflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}" -asmflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}"

.PHONY: all setup build-linux build-osx build-windows build-rust build-rust-cross build-freebsd reg-rust
all: setup build-linux build-freebsd build-osx build-windows

build-rust:
	cd rust && CARGO_TARGET_DIR=$$PWD/target cargo build --release
	mkdir -p bin/osx bin/linux bin/windows
	cp rust/target/release/uploader bin/osx/uploader-rust-darwin-$$(uname -m) 2>/dev/null || true
	@ls -lh rust/target/release/uploader

build-rust-cross:
	bash scripts/cross-rust.sh

reg-rust:
	bash scripts/reg-rust.sh

Name := uploader

setup:
	mkdir -p bin/linux
	mkdir -p bin/freebsd
	mkdir -p bin/osx
	mkdir -p bin/windows

build-linux:
	${BUILD_ENV} GOARCH=amd64 GOOS=linux go build ${LDFLAGS-Linux} -trimpath -o bin/linux/${Name}-linux-amd64 main.go;
	${BUILD_ENV} GOARCH=386 GOOS=linux go build ${LDFLAGS-Linux} -trimpath -o bin/linux/${Name}-linux-x86 main.go;
	${BUILD_ENV} GOARCH=arm64 GOOS=linux go build ${LDFLAGS-Linux} -trimpath -o bin/linux/${Name}-linux-arm64 main.go;

build-freebsd:
	${BUILD_ENV} GOARCH=amd64 GOOS=freebsd go build ${LDFLAGS-Linux} -trimpath -o bin/freebsd/${Name}-freebsd-amd64 main.go;
	${BUILD_ENV} GOARCH=386 GOOS=freebsd go build ${LDFLAGS-Linux} -trimpath -o bin/freebsd/${Name}-freebsd-x86 main.go;

build-osx:
	${BUILD_ENV} GOARCH=amd64 GOOS=darwin go build ${LDFLAGS-Linux} -trimpath -o bin/osx/${Name}-darwin-amd64 main.go;
	${BUILD_ENV} GOARCH=arm64 GOOS=darwin go build ${LDFLAGS-Linux} -trimpath -o bin/osx/${Name}-darwin-arm64 main.go;

build-windows:
	${BUILD_ENV} GOARCH=amd64 GOOS=windows go build ${LDFLAGS-Win} -trimpath -o bin/windows/${Name}-windows-amd64.exe main.go;
	${BUILD_ENV} GOARCH=386 GOOS=windows go build ${LDFLAGS-Win} -trimpath -o bin/windows/${Name}-windows-x86.exe main.go;
	${BUILD_ENV} GOARCH=arm64 GOOS=windows go build ${LDFLAGS-Win} -trimpath -o bin/windows/${Name}-windows-arm64.exe main.go;
