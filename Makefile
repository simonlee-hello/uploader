# 动态编译
BUILD_ENV := CGO_ENABLED=0
LDFLAGS-Linux := -ldflags '-s -w -extldflags "-static"' -gcflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}" -asmflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}"
#LDFLAGS-Win=-v -a -ldflags '-s -w -H windowsgui -extldflags "-static"' -gcflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}" -asmflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}"
LDFLAGS-Win := -ldflags '-s -w -extldflags "-static"' -gcflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}" -asmflags="all=-trimpath=${PWD};${GOPATH};${GOROOT}"


.PHONY: all setup build-linux build-osx build-windows
all: setup build-linux build-freebsd build-osx build-windows

Name := uploader

setup:
	mkdir -p bin/linux
	mkdir -p bin/freebsd
	mkdir -p bin/osx
	mkdir -p bin/windows

build-linux:
	${BUILD_ENV} GOARCH=amd64 GOOS=linux go build ${LDFLAGS-Linux} -o bin/linux/${Name}-linux-amd64 main.go;
	${BUILD_ENV} GOARCH=386 GOOS=linux go build ${LDFLAGS-Linux} -o bin/linux/${Name}-linux-x86 main.go;

build-freebsd:
	${BUILD_ENV} GOARCH=amd64 GOOS=freebsd go build ${LDFLAGS-Linux} -o bin/freebsd/${Name}-freebsd-amd64 main.go;
	${BUILD_ENV} GOARCH=386 GOOS=freebsd go build ${LDFLAGS-Linux} -o bin/freebsd/${Name}-freebsd-x86 main.go;

build-osx:
	${BUILD_ENV} GOARCH=amd64 GOOS=darwin go build ${LDFLAGS-Linux} -o bin/osx/${Name}-darwin-amd64 main.go;

build-windows:
	${BUILD_ENV} GOARCH=amd64 GOOS=windows go build ${LDFLAGS-Win} -o bin/windows/${Name}-windows-amd64.exe main.go;
	${BUILD_ENV} GOARCH=386 GOOS=windows go build ${LDFLAGS-Win} -o bin/windows/${Name}-windows-x86.exe main.go;
