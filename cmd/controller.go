package cmd

import (
	"fmt"

	"strings"

	"github.com/spf13/cobra"

	"uploader/apis"
	fichier "uploader/apis/public/1fichier"
	"uploader/apis/public/airportal"
	"uploader/apis/public/catbox"
	"uploader/apis/public/cowtransfer"
	"uploader/apis/public/downloadgg"
	"uploader/apis/public/fileio"
	"uploader/apis/public/gofile"
	"uploader/apis/public/infura"
	"uploader/apis/public/lanzous"
	"uploader/apis/public/litterbox"
	"uploader/apis/public/null"
	"uploader/apis/public/tmplink"
	"uploader/apis/public/wenshushu"
	//"uploader/apis/public/wetransfer"
)

var (
	backendList = [][]any{
		{"arp", "airportal", airportal.Backend},
		{"cat", "catbox", catbox.Backend},
		{"cow", "cowtransfer", cowtransfer.Backend},
		{"fic", "1fichier", fichier.Backend},
		{"fio", "file.io", fileio.Backend},
		{"gg", "downloadgg", downloadgg.Backend},
		{"gof", "gofile", gofile.Backend},
		{"inf", "infura", infura.Backend},
		{"lit", "littlebox", litterbox.Backend},
		{"lzs", "lanzous", lanzous.Backend},
		{"nil", "null", null.Backend},
		{"tmp", "tmplink", tmplink.Backend},
		//{"wet", "wetransfer", wetransfer.Backend},
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
