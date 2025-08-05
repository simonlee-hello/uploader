package cmd

import (
	"fmt"
	"uploader/apis/public/anonfiles"
	"uploader/apis/public/temp"

	"strings"

	"github.com/spf13/cobra"

	"uploader/apis"
	fichier "uploader/apis/public/1fichier"
	"uploader/apis/public/catbox"
	"uploader/apis/public/downloadgg"
	"uploader/apis/public/gofile"
	"uploader/apis/public/litterbox"
	"uploader/apis/public/null"
	"uploader/apis/public/wenshushu"
)

var (
	backendList = [][]any{
		{"fic", "1fichier", fichier.Backend},
		{"anon", "anonfiles", anonfiles.Backend},
		{"cat", "catbox", catbox.Backend},
		{"gg", "downloadgg", downloadgg.Backend},
		{"gof", "gofile", gofile.Backend},
		{"lit", "littlebox", litterbox.Backend},
		{"nil", "null", null.Backend},
		{"temp", "temp", temp.Backend},
		{"wss", "wenshushu", wenshushu.Backend},
	}
)

func inList(list []string, item string) bool {
	for _, i := range list {
		if i == item {
			return true
		}
	}
	return false
}

func runner(backend apis.BaseBackend) func(cmd *cobra.Command, args []string) {
	return func(cmd *cobra.Command, args []string) {
		if len(args) == 0 {
			_ = cmd.Help()
		}

		file := uploadWalker(args)
		if len(file) > 0 {
			apis.Upload(file, backend)
		}

		for k, item := range args {
			isCommand := false
			if strings.HasPrefix(item, "-") {
				isCommand = true
			}
			if k > 1 {
				if strings.HasPrefix(args[k-1], "-") {
					isCommand = true
				}
			}
			if !inList(file, item) && !isCommand {
				fmt.Printf("transfer: %s: No such file, link or directory\n", item)
			}
		}

	}
}
