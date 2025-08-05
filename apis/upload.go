package apis

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sync"
)

func Upload(files []string, backend BaseBackend) {
	tmpOut := os.Stdout
	if MuteMode {
		transferConfig.NoBarMode = true
		os.Stdout, _ = os.Open(os.DevNull)
	}
	var (
		sizes []int64
		paths []string
	)
	for _, v := range files {
		err := filepath.Walk(v, func(path string, info os.FileInfo, err error) error {
			if info.IsDir() {
				return nil
			}
			if err != nil {
				return err
			}
			paths = append(paths, path)
			sizes = append(sizes, info.Size())
			return nil
		})
		if err != nil {
			_, _ = fmt.Fprintf(os.Stderr, "filepath.walk failed: %v, onfile: %s\n", err, v)
			return
		}
	}
	err := backend.InitUpload(paths, sizes)
	if err != nil {
		_, _ = fmt.Fprintf(os.Stderr, "Error occurred during initialization:\n  %s\n", err)
		return
	}
	for n, file := range paths {
		ps, _ := filepath.Abs(file)
		fmt.Printf("Local: %s\n", ps)
		resp, err := upload(file, sizes[n], backend)
		if err != nil {
			_, _ = fmt.Fprintf(os.Stderr, "Error occurred during upload %s:\n  %s\n", file, err)
		}
		if resp != "" && MuteMode {
			_, _ = fmt.Fprintln(tmpOut, resp)
		}
		if resp != "" && Output != "" {

			f, err := os.OpenFile(Output, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
			if err != nil {
				_, _ = fmt.Fprintf(os.Stderr, "Error opening output file %s: %v\n", Output, err)
			} else {
				defer f.Close()
				if _, err := f.Write([]byte(resp + "\n")); err != nil {
					_, _ = fmt.Fprintf(os.Stderr, "Error appending output to file %s: %v\n", Output, err)
				} else {
					fmt.Printf("Output appended to %s\n", Output)
				}
			}
		}
	}
	resp, err := backend.FinishUpload(files)
	if err != nil {
		_, _ = fmt.Fprintf(os.Stderr, "Error occurred during finalizing upload:\n  %s\n", err)
	}
	if resp != "" && MuteMode {
		_, _ = fmt.Fprintln(tmpOut, resp)
	}
}

func monitor(w *io.PipeWriter, sig *sync.WaitGroup) {
	sig.Wait()
	_ = w.Close()
}

func upload(file string, size int64, backend BaseBackend) (string, error) {
	info, err := os.Stat(file)
	if err != nil {
		return "", fmt.Errorf("stat file %s failed: %s", file, err)
	}
	err = backend.PreUpload(info.Name(), size)
	if err != nil {
		return "", fmt.Errorf("start upload failed: %s", err)
	}
	fileStream, err := os.Open(file)
	if err != nil {
		return "", fmt.Errorf("open %s failed: %s", file, err)
	}
	var reader io.Reader

	reader = fileStream
	if !transferConfig.NoBarMode {
		reader = backend.StartProgress(fileStream, size)
	}

	err = backend.DoUpload(info.Name(), size, reader)
	if err != nil {
		return "", fmt.Errorf("upload error: %s", err)
	}
	_ = fileStream.Close()
	if !transferConfig.NoBarMode {
		backend.EndProgress()
	}
	resp, err := backend.PostUpload(info.Name(), size)
	if err != nil {
		return "", fmt.Errorf("postUpload error: %s", err)
	}
	return resp, nil
}
