package cnet

import (
	"fmt"
	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(cnet)
)

type cnet struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *cnet) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("cnet - https://paste.c-net.org/\n\n" +
		utils.Spacer("  Size Limit: 50M\n"))
}
