package cmd

import (
	"fmt"
	"strings"

	"uploader/utils"
)

func uploadWalker(items []string) ([]string, error) {
	var uploadList []string
	var missing []string
	for _, v := range items {
		if utils.IsExist(v) {
			uploadList = append(uploadList, v)
		} else {
			missing = append(missing, v)
		}
	}
	if len(missing) > 0 {
		return nil, fmt.Errorf("not found: %s", strings.Join(missing, ", "))
	}
	return uploadList, nil
}
