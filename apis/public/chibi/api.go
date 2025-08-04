package chibi

import (
	"fmt"

	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(chibi)
)

type chibi struct {
	apis.Backend
	resp     string
	Commands [][]string
}

func (b *chibi) SetArgs(cmd *cobra.Command) {
	cmd.Long = fmt.Sprintf("Chibisafe/Lolisafe\n\n" +
		utils.Spacer("  Repo: https://github.com/WeebDev/chibisafe\n"))
}
