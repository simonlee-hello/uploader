package downloadgg

import (
	"fmt"
	"uploader/utils"

	"github.com/spf13/cobra"
	"uploader/apis"
)

var (
	Backend = new(dlg)
)

type dlg struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *dlg) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("dlg - https://download.gg/\n\n" +
		utils.Spacer("  Size Limit: 25G\n"))
}
