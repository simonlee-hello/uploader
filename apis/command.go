package apis

import (
	"github.com/spf13/cobra"
	"uploader/apis/methods"
)

var (
	transferConfig methods.TransferConfig
	DebugMode      bool
	MuteMode       bool
)

func InitCmd(cmd *cobra.Command) {
	cmd.PersistentFlags().BoolVarP(&transferConfig.CryptoMode,
		"encrypt", "", false, "encrypt stream when upload")
	cmd.PersistentFlags().StringVarP(&transferConfig.CryptoKey,
		"encrypt-key", "", "", "specify the encrypt key")
	cmd.PersistentFlags().BoolVarP(&transferConfig.NoBarMode,
		"no-progress", "", false, "disable progress bar to reduce output")
	cmd.PersistentFlags().BoolVarP(&MuteMode,
		"silent", "", false, "enable silent mode to mute output")
	cmd.PersistentFlags().BoolVarP(&DebugMode,
		"verbose", "v", false, "enable verbose mode to debug")
	// workround
	transferConfig.DebugMode = &DebugMode

	cmd.HelpFunc()
}
