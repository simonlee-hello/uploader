package apis

import (
	"github.com/cheggaaa/pb/v3"
	"os"
)

type writeCounter struct {
	bar    *pb.ProgressBar
	offset int64
	writer *os.File
}

func (wc *writeCounter) Write(p []byte) (int, error) {
	n, err := wc.writer.WriteAt(p, wc.offset)
	if err != nil {
		return 0, err
	}
	wc.offset += int64(n)
	if wc.bar != nil {
		wc.bar.Add(n)
	}
	return n, nil
}
