package cmd

import (
	"bufio"
	"fmt"
	"os"
	"strings"
	fichier "uploader/apis/public/1fichier"

	"uploader/apis"

	"github.com/spf13/cobra"
)

var (
	rootCmd = &cobra.Command{
		Use:           "uploader",
		Short:         "uploader is a very simple big file uploader tool",
		SilenceErrors: true,
		Run: func(cmd *cobra.Command, args []string) {
			if VersionMode {
				fmt.Printf("uploader version: 1.0.0\n")
				os.Exit(0)
			} else {
				_ = cmd.Help()
			}
		},
	}

	VersionMode bool
	KeepMode    bool
)

func init() {
	rootCmd.PersistentFlags().BoolVarP(&VersionMode,
		"version", "", false, "show version and exit")
	rootCmd.PersistentFlags().BoolVarP(&KeepMode,
		"keep", "", false, "keep program active when process finish")
	apis.InitCmd(rootCmd)
	for _, item := range backendList {
		backend := item[len(item)-1].(apis.BaseBackend)
		var alias []string
		for _, a := range item {
			if _, ok := a.(string); ok {
				alias = append(alias, a.(string))
			}
		}
		backendCmd := &cobra.Command{
			Use:     alias[0],
			Aliases: alias[1:],
			Short:   fmt.Sprintf("Use %s API to uploader file", alias[1]),
			Run:     runner(backend),
		}
		backend.SetArgs(backendCmd)
		backendCmd.Hidden = true
		rootCmd.AddCommand(backendCmd)
	}
}

func Execute() {

	defer func() {
		if KeepMode {
			fmt.Print("Press the enter key to exit...")
			reader := bufio.NewReader(os.Stdin)
			_, _ = reader.ReadString('\n')
		}
	}()

	if err := rootCmd.Execute(); err != nil {
		if strings.HasPrefix(err.Error(), "unknown command") {
			handleRootTransfer(os.Args[1:])
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	}
}

func handleRootTransfer(args []string) {

	rootCmd.ParseFlags(args)
	files := uploadWalker(args)
	if len(files) != 0 {
		if !apis.MuteMode {
			fmt.Println("Warning: backend is not set. Default using 1fichier api")
			fmt.Printf("Run 'uploader --help' for usage.\n\n")
		}
		runner(fichier.Backend)(rootCmd, args)
	}
}
