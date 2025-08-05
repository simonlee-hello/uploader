package tempfiles

import (
	"fmt"
	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(tempf)
)

type tempf struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *tempf) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("tempf - https://tmpfiles.org/api/v1/upload\n\n" +
		utils.Spacer("  Size Limit: 100M\n"))
}
