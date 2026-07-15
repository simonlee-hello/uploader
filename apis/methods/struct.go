package methods

import (
	"io"
)

type MultiPartUploadConfig struct {
	Debug      bool
	Endpoint   string
	FileName   string
	FileReader io.Reader
	FileSize   int64
	Headers    map[string]string
}

type TransferConfig struct {
	Parallel   int
	DebugMode  *bool
	NoBarMode  bool
	CryptoMode bool
	CryptoKey  string
}
