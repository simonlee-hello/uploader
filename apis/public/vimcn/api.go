package vimcn

import (
	"fmt"

	"github.com/spf13/cobra"
	"uploader/apis"
	"uploader/utils"
)

var (
	Backend = new(vimcn)
)

type vimcn struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *vimcn) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("vim-cn - https://img.vim-cn.com/\n\n" +
		utils.Spacer("  Size Limit: 100M\n") +
		utils.Spacer("  Upload Service: Cloudflare, Global\n") +
		utils.Spacer("  Download Service: Cloudflare, Global\n"))
}
