package cmd

import (
	"fmt"
	"os"

	"uploader/apis"
	"uploader/route"
)

func uploadWithOptions(files []string, primary string, auto bool, force bool) error {
	backend := ""
	if !auto {
		backend = primary
	}
	cfg := apis.TransferConfig()
	link, _, err := route.UploadWithOptions(files, route.Options{
		Backend: backend,
		Force:   force,
		Quiet:   apis.QuietMode,
		Mute:    apis.MuteMode,
		Encrypt: cfg.CryptoMode,
		Key:     cfg.CryptoKey,
		OnSuccess: func(name string) {
			saveLastBackend(name)
		},
	})
	if err != nil {
		return err
	}
	// Single-file quiet path swallows PostUpload prints; emit the returned link.
	if link != "" && apis.MuteMode {
		fmt.Fprintln(os.Stdout, link)
	}
	return nil
}
