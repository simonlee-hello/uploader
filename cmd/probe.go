package cmd

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"uploader/apis"
)

type probeResult struct {
	Name    string
	OK      bool
	Latency time.Duration
	Link    string
	Err     string
	Skipped bool
}

func runProbe(args []string) {
	var (
		all        bool
		help       bool
		parallel   int
		timeoutSec float64
	)
	args = reorderArgs(args, map[string]bool{"-timeout": true, "-parallel": true})
	fs := flag.NewFlagSet("probe", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	fs.BoolVar(&all, "all", false, "include disabled backends")
	fs.BoolVar(&help, "h", false, "help")
	fs.BoolVar(&help, "help", false, "help")
	fs.IntVar(&parallel, "parallel", 3, "concurrency")
	fs.Float64Var(&timeoutSec, "timeout", 45, "per-backend timeout sec")
	if err := fs.Parse(args); err != nil {
		os.Exit(2)
	}
	if help {
		fmt.Fprintln(os.Stderr, "usage: uploader probe [-all] [-parallel N] [-timeout SEC] [backend...]")
		return
	}
	if parallel < 1 {
		parallel = 1
	}

	targets := selectProbeTargets(fs.Args(), all)
	if len(targets) == 0 {
		fmt.Fprintln(os.Stderr, "no backends")
		os.Exit(1)
	}

	probeFile, err := writeProbeFile()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer os.Remove(probeFile)

	fmt.Fprintf(os.Stderr, "probing %d backend(s), parallel=%d timeout=%.0fs\n\n", len(targets), parallel, timeoutSec)

	results := make([]probeResult, len(targets))
	sem := make(chan struct{}, parallel)
	var wg sync.WaitGroup
	var printMu sync.Mutex

	for i, info := range targets {
		i, info := i, info
		wg.Add(1)
		go func() {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			results[i] = probeOne(info, probeFile, time.Duration(timeoutSec*float64(time.Second)))
			status := "FAIL"
			if results[i].OK {
				status = "OK"
			}
			if results[i].Skipped {
				status = "SKIP"
			}
			extra := results[i].Err
			if results[i].OK {
				extra = shortLink(results[i].Link)
			}
			printMu.Lock()
			fmt.Printf("%-4s %-6s %8s  %s\n", status, results[i].Name, formatLatency(results[i].Latency), extra)
			printMu.Unlock()
		}()
	}
	wg.Wait()

	sort.SliceStable(results, func(i, j int) bool {
		a, b := results[i], results[j]
		if a.OK != b.OK {
			return a.OK
		}
		if a.Skipped != b.Skipped {
			return !a.Skipped
		}
		if a.OK && b.OK {
			return a.Latency < b.Latency
		}
		return a.Name < b.Name
	})

	fmt.Fprintln(os.Stderr, "\nsummary (prefer top successes):")
	fmt.Printf("%-6s %-6s %8s  %s\n", "NAME", "RESULT", "TIME", "DETAIL")
	for _, r := range results {
		res, detail := "fail", r.Err
		if r.Skipped {
			res = "skip"
		} else if r.OK {
			res = "ok"
			detail = shortLink(r.Link)
		}
		fmt.Printf("%-6s %-6s %8s  %s\n", r.Name, res, formatLatency(r.Latency), detail)
	}

	for _, r := range results {
		if r.OK {
			fmt.Fprintf(os.Stderr, "\nrecommended: uploader -b %s <file>\n", r.Name)
			return
		}
	}
	fmt.Fprintln(os.Stderr, "\nno working backend for this network")
}

func selectProbeTargets(names []string, all bool) []BackendInfo {
	if len(names) > 0 {
		var out []BackendInfo
		for _, n := range names {
			info := findBackend(n)
			if info == nil {
				fmt.Fprintf(os.Stderr, "unknown backend: %s\n", n)
				continue
			}
			out = append(out, *info)
		}
		return out
	}
	var out []BackendInfo
	for _, b := range backends {
		if !all && b.Status == "down" {
			continue
		}
		out = append(out, b)
	}
	return out
}

func writeProbeFile() (string, error) {
	path := filepath.Join(os.TempDir(), fmt.Sprintf("uploader-probe-%d.txt", time.Now().UnixNano()))
	return path, os.WriteFile(path, []byte("uploader probe\n"), 0644)
}

var probeMu sync.Mutex

func probeOne(info BackendInfo, file string, timeout time.Duration) probeResult {
	res := probeResult{Name: info.Name}
	if info.Status == "down" {
		res.Skipped = true
		res.Err = info.Note
		return res
	}

	type outcome struct {
		link    string
		err     error
		latency time.Duration
	}
	ch := make(chan outcome, 1)
	go func() {
		probeMu.Lock()
		start := time.Now()
		link, err := probeUpload(info, file)
		lat := time.Since(start)
		probeMu.Unlock()
		ch <- outcome{link: link, err: err, latency: lat}
	}()

	timer := time.NewTimer(timeout)
	defer timer.Stop()
	select {
	case <-timer.C:
		res.Latency = timeout
		res.Err = fmt.Sprintf("timeout >%.0fs", timeout.Seconds())
		return res
	case out := <-ch:
		res.Latency = out.latency
		if out.err != nil {
			res.Err = truncateErr(out.err.Error(), 72)
			return res
		}
		res.OK = true
		res.Link = out.link
		return res
	}
}

func probeUpload(info BackendInfo, file string) (string, error) {
	oldMute, oldDebug := apis.MuteMode, apis.DebugMode
	oldOut := apis.Output
	oldCfg := *apis.TransferConfig()
	oldStdout := os.Stdout
	defer func() {
		apis.MuteMode = oldMute
		apis.DebugMode = oldDebug
		apis.Output = oldOut
		*apis.TransferConfig() = oldCfg
		os.Stdout = oldStdout
	}()

	devNull, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err == nil {
		os.Stdout = devNull
		defer devNull.Close()
	}

	apis.MuteMode = false
	apis.DebugMode = false
	apis.Output = ""
	cfg := apis.TransferConfig()
	cfg.NoBarMode = true
	cfg.CryptoMode = false
	cfg.CryptoKey = ""
	cfg.MaxBytes = info.MaxBytes()
	cfg.BackendName = info.Name
	cfg.RecursiveDirs = false

	applyBackendOptions(info.Name)
	return apis.UploadFile(file, info.Backend)
}

func formatLatency(d time.Duration) string {
	if d <= 0 {
		return "-"
	}
	if d < time.Second {
		return fmt.Sprintf("%dms", d.Milliseconds())
	}
	return fmt.Sprintf("%.1fs", d.Seconds())
}

func shortLink(link string) string {
	link = strings.TrimSpace(link)
	if link == "" {
		return "(no link)"
	}
	if len(link) > 64 {
		return link[:61] + "..."
	}
	return link
}

func truncateErr(s string, n int) string {
	s = strings.ReplaceAll(s, "\n", " ")
	if len(s) <= n {
		return s
	}
	return s[:n-3] + "..."
}
