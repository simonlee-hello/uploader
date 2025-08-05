package bashupload

import (
	"fmt"
	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(bash)
)

type bash struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *bash) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("bash - https://bashupload.com/\n\n" +
		utils.Spacer("  Size Limit: 50GB\n") +
		utils.Spacer("  Upload Service: Files are stored for 3 days and can be downloaded only once.\n"))
}
