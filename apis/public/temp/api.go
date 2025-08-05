package temp

import (
	"fmt"
	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(temp)
)

type temp struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *temp) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("temp - https://temp.sh/\n\n" +
		utils.Spacer("  Size Limit: 4GB\n"))
}
