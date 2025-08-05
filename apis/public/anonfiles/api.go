package anonfiles

import (
	"fmt"
	"uploader/utils"

	"github.com/spf13/cobra"
	"uploader/apis"
)

var (
	Backend = new(anon)
)

type anon struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *anon) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("anon - https://anonymfile.com\n\n" +
		utils.Spacer("  Size Limit: 5000MB\n"))
}
