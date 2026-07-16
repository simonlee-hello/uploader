package cmd

import (
	"bufio"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

type userConfig struct {
	Backend string
	Auto    bool
}

func configDir() string {
	if v := os.Getenv("UPLOADER_CONFIG_DIR"); v != "" {
		return v
	}
	if runtime.GOOS == "windows" {
		if app := os.Getenv("APPDATA"); app != "" {
			return filepath.Join(app, "uploader")
		}
	}
	if xdg := os.Getenv("XDG_CONFIG_HOME"); xdg != "" {
		return filepath.Join(xdg, "uploader")
	}
	if home, err := os.UserHomeDir(); err == nil {
		return filepath.Join(home, ".config", "uploader")
	}
	return ""
}

func loadUserConfig() userConfig {
	var cfg userConfig
	dir := configDir()
	if dir == "" {
		return cfg
	}
	f, err := os.Open(filepath.Join(dir, "config"))
	if err != nil {
		return cfg
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		k, v, ok := strings.Cut(line, "=")
		if !ok {
			continue
		}
		switch strings.TrimSpace(k) {
		case "backend":
			cfg.Backend = strings.TrimSpace(v)
		case "auto":
			cfg.Auto = strings.EqualFold(strings.TrimSpace(v), "true") || strings.TrimSpace(v) == "1"
		}
	}
	return cfg
}

func resolveDefaultBackend() string {
	if v := strings.TrimSpace(os.Getenv("UPLOADER_BACKEND")); v != "" {
		return v
	}
	if cfg := loadUserConfig(); cfg.Backend != "" {
		return cfg.Backend
	}
	if last := readLastBackend(); last != "" {
		return last
	}
	return "temp"
}

func readLastBackend() string {
	dir := configDir()
	if dir == "" {
		return ""
	}
	b, err := os.ReadFile(filepath.Join(dir, "last-backend"))
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(b))
}

func saveLastBackend(name string) {
	dir := configDir()
	if dir == "" || name == "" {
		return
	}
	_ = os.MkdirAll(dir, 0755)
	_ = os.WriteFile(filepath.Join(dir, "last-backend"), []byte(name+"\n"), 0644)
}

func configAutoEnabled() bool {
	return loadUserConfig().Auto
}
