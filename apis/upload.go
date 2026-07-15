package apis

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"uploader/crypto"
)

func Upload(files []string, backend BaseBackend) {
	tmpOut := os.Stdout
	if MuteMode {
		transferConfig.NoBarMode = true
		os.Stdout, _ = os.Open(os.DevNull)
		defer func() { os.Stdout = tmpOut }()
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
			fmt.Fprintf(os.Stderr, "walk: %v\n", err)
			return
		}
	}

	if transferConfig.CryptoMode {
		displayKey, normalized, err := crypto.NormalizeKey(transferConfig.CryptoKey, true)
		if err != nil {
			fmt.Fprintf(os.Stderr, "encrypt: %v\n", err)
			return
		}
		transferConfig.CryptoKey = normalized
		fmt.Fprintf(os.Stderr, "key: %s\n", displayKey)
		for i := range sizes {
			sizes[i] = crypto.CalcEncryptSize(sizes[i])
		}
	}

	if err := backend.InitUpload(paths, sizes); err != nil {
		fmt.Fprintf(os.Stderr, "init: %v\n", err)
		return
	}
	for n, file := range paths {
		resp, err := upload(file, sizes[n], backend)
		if err != nil {
			fmt.Fprintf(os.Stderr, "upload %s: %v\n", filepath.Base(file), err)
		}
		if resp != "" && MuteMode {
			fmt.Fprintln(tmpOut, resp)
		}
		if resp != "" && Output != "" {
			f, err := os.OpenFile(Output, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
			if err != nil {
				fmt.Fprintf(os.Stderr, "result file: %v\n", err)
			} else {
				_, _ = f.Write([]byte(resp + "\n"))
				_ = f.Close()
			}
		}
	}
	resp, err := backend.FinishUpload(files)
	if err != nil {
		fmt.Fprintf(os.Stderr, "finish: %v\n", err)
	}
	if resp != "" && MuteMode {
		fmt.Fprintln(tmpOut, resp)
	}
}

func UploadFile(path string, backend BaseBackend) (string, error) {
	info, err := os.Stat(path)
	if err != nil {
		return "", err
	}
	size := info.Size()
	nameSize := size
	if transferConfig.CryptoMode {
		nameSize = crypto.CalcEncryptSize(size)
	}
	if err := backend.InitUpload([]string{path}, []int64{nameSize}); err != nil {
		return "", err
	}
	link, err := upload(path, size, backend)
	if err != nil {
		return "", err
	}
	finish, err := backend.FinishUpload([]string{path})
	if err != nil {
		return link, err
	}
	if finish != "" {
		return finish, nil
	}
	return link, nil
}

func upload(file string, size int64, backend BaseBackend) (string, error) {
	info, err := os.Stat(file)
	if err != nil {
		return "", err
	}

	name := info.Name()
	uploadSize := size
	if transferConfig.CryptoMode {
		uploadSize = crypto.CalcEncryptSize(size)
		name = name + ".encrypt"
	}

	if err = backend.PreUpload(name, uploadSize); err != nil {
		return "", err
	}
	fileStream, err := os.Open(file)
	if err != nil {
		return "", err
	}
	defer fileStream.Close()

	var reader io.Reader = fileStream
	if transferConfig.CryptoMode {
		pr, pw := io.Pipe()
		go func() {
			encErr := crypto.StreamEncrypt(fileStream, pw, transferConfig.CryptoKey, 0)
			_ = pw.CloseWithError(encErr)
		}()
		reader = pr
	}

	if !transferConfig.NoBarMode {
		reader = backend.StartProgress(reader, uploadSize)
	}

	if err = backend.DoUpload(name, uploadSize, reader); err != nil {
		return "", err
	}
	if !transferConfig.NoBarMode {
		backend.EndProgress()
	}
	return backend.PostUpload(name, uploadSize)
}
