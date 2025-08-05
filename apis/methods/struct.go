package methods

import (
	"github.com/cheggaaa/pb/v3"
	"io"
	"os"
)

type MultiPartUploadConfig struct {
	Debug      bool
	Endpoint   string
	FileName   string
	FileReader io.Reader
	FileSize   int64
}

type TransferConfig struct {
	Parallel   int
	DebugMode  *bool
	NoBarMode  bool
	CryptoMode bool
	CryptoKey  string
}

type writeCounter struct {
	bar    *pb.ProgressBar
	offset int64
	writer *os.File
}
