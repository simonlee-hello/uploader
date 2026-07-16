package cmd

import (
	"fmt"
	"os"
	"strings"

	"uploader/apis"
)

// autoBackendOrder is the preferred failover sequence (ok backends only).
var autoBackendOrder = []string{"temp", "lit", "gof", "gg", "fic", "tmpf", "cnet", "wss"}

func backendAllowed(info *BackendInfo, force bool) error {
	if info == nil {
		return fmt.Errorf("unknown backend")
	}
	switch info.Status {
	case "down":
		if !force {
			return fmt.Errorf("backend %s is down (%s); use -force to try anyway", info.Name, info.Note)
		}
	case "flaky":
		if !force {
			return fmt.Errorf("backend %s is flaky (%s); use -force to try anyway", info.Name, info.Note)
		}
	}
	return nil
}

func buildAutoCandidates(primary string, maxSize int64, force bool) []*BackendInfo {
	seen := map[string]bool{}
	var out []*BackendInfo
	add := func(name string) {
		if seen[name] {
			return
		}
		info := findBackend(name)
		if info == nil {
			return
		}
		if info.Status == "down" && !force {
			return
		}
		if info.Status == "flaky" && !force {
			return
		}
		lim := info.MaxBytes()
		if maxSize > 0 && lim > 0 && maxSize > lim {
			return
		}
		seen[name] = true
		out = append(out, info)
	}
	if primary != "" {
		add(primary)
	}
	for _, name := range autoBackendOrder {
		add(name)
	}
	return out
}

func setupUploadFor(info *BackendInfo) {
	applyBackendOptions(info.Name)
	cfg := apis.TransferConfig()
	cfg.MaxBytes = info.MaxBytes()
	cfg.BackendName = info.Name
	apis.SizeHint = func(size int64) string {
		alts := backendsFitting(size)
		var filtered []string
		for _, a := range alts {
			if a != info.Name && backendStatus(a) == "ok" {
				filtered = append(filtered, a)
			}
		}
		if len(filtered) == 0 {
			return ""
		}
		if len(filtered) > 6 {
			filtered = filtered[:6]
		}
		return "try: -b " + strings.Join(filtered, " | -b ")
	}
}

func backendStatus(name string) string {
	if info := findBackend(name); info != nil {
		return info.Status
	}
	return ""
}

func uploadWithOptions(files []string, primary string, auto bool, force bool) error {
	maxSize, err := estimateUploadMaxSize(files)
	if err != nil {
		return err
	}

	var candidates []*BackendInfo
	if auto {
		candidates = buildAutoCandidates(primary, maxSize, force)
	} else if primary != "" {
		info := findBackend(primary)
		if info == nil {
			return fmt.Errorf("unknown backend %q", primary)
		}
		if err := backendAllowed(info, force); err != nil {
			return err
		}
		candidates = []*BackendInfo{info}
	} else {
		def := resolveDefaultBackend()
		info := findBackend(def)
		if info == nil {
			return fmt.Errorf("unknown default backend %q", def)
		}
		if err := backendAllowed(info, force); err != nil {
			return err
		}
		candidates = []*BackendInfo{info}
	}

	if len(candidates) == 0 {
		return fmt.Errorf("no backend available for this file size")
	}

	var lastErr error
	for i, info := range candidates {
		if !apis.QuietMode && auto && i > 0 {
			fmt.Fprintf(os.Stderr, "retry backend %s...\n", info.Name)
		}
		setupUploadFor(info)
		lastErr = apis.Upload(files, info.Backend)
		if lastErr == nil {
			saveLastBackend(info.Name)
			return nil
		}
		if !auto {
			return lastErr
		}
		if !apis.QuietMode {
			fmt.Fprintf(os.Stderr, "backend %s failed: %v\n", info.Name, lastErr)
		}
	}
	return lastErr
}
