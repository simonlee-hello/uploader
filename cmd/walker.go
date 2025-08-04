package cmd

import "uploader/utils"

func uploadWalker(items []string) []string {
	var uploadList []string
	for _, v := range items {
		if utils.IsExist(v) {
			uploadList = append(uploadList, v)
		}
	}
	return uploadList
}
