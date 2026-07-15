package apis

import (
	"uploader/apis/methods"
)

var (
	transferConfig methods.TransferConfig
	DebugMode      bool
	MuteMode       bool
	Output         string
)

func TransferConfig() *methods.TransferConfig {
	return &transferConfig
}

func init() {
	transferConfig.DebugMode = &DebugMode
}
