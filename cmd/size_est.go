package cmd

import "uploader/route"

func estimateUploadMaxSize(files []string) (int64, error) {
	return route.EstimateMaxSize(files)
}
