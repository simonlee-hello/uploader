package cmd

import (
	"os"
	"path/filepath"

	"uploader/apis"
	"uploader/crypto"
)

func estimateUploadMaxSize(files []string) (int64, error) {
	cfg := apis.TransferConfig()
	var max int64
	for _, v := range files {
		info, err := os.Stat(v)
		if err != nil {
			return 0, err
		}
		if info.IsDir() && !cfg.RecursiveDirs {
			// Zip size unknown without packing; use dir file sum as upper bound.
			var sum int64
			err = filepath.Walk(v, func(path string, fi os.FileInfo, err error) error {
				if err != nil || fi.IsDir() {
					return err
				}
				sum += fi.Size()
				return nil
			})
			if err != nil {
				return 0, err
			}
			if sum > max {
				max = sum
			}
			continue
		}
		err = filepath.Walk(v, func(path string, fi os.FileInfo, err error) error {
			if err != nil || fi.IsDir() {
				return err
			}
			sz := fi.Size()
			if cfg.CryptoMode {
				sz = crypto.CalcEncryptSize(sz)
			}
			if sz > max {
				max = sz
			}
			return nil
		})
		if err != nil {
			return 0, err
		}
	}
	return max, nil
}
