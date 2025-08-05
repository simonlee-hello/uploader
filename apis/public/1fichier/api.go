package fichier

import (
	"fmt"
	"uploader/apis"
	"uploader/utils"

	"github.com/spf13/cobra"
)

var (
	Backend = new(fichier)
)

type fichier struct {
	apis.Backend
	pwd    string
	apiKey string
	email  string
	useFTP bool
	resp   string
	remove string
}

func (b *fichier) SetArgs(cmd *cobra.Command) {
	cmd.Flags().StringVarP(&b.pwd, "password", "p", "", "Set the download password")
	cmd.Flags().StringVarP(&b.apiKey, "apiKey", "k", "", "Set the user api key")
	cmd.Flags().StringVarP(&b.email, "email", "e", "", "Set the email to receive an notification, only valid with apiKey")
	cmd.Flags().BoolVarP(&b.useFTP, "ftp", "f", false, "Upload via FTP")

	cmd.Long = fmt.Sprintf("1fichier - https://1fichier.com/\n\n" +
		utils.Spacer("  Size Limit: 300G\n") +
		utils.Spacer("  Upload Service: DSTORAGE s.a.s.\n"))
}
