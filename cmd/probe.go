package cmd

import (
	"flag"
	"fmt"
	"os"
	"time"

	"uploader/route"
)

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

	targets := route.SelectProbeTargets(fs.Args(), all)
	if len(targets) == 0 {
		fmt.Fprintln(os.Stderr, "no backends")
		os.Exit(1)
	}

	fmt.Fprintf(os.Stderr, "probing %d backend(s), parallel=%d timeout=%.0fs\n\n", len(targets), parallel, timeoutSec)

	results, err := route.ProbeAll(targets, parallel, time.Duration(timeoutSec*float64(time.Second)), true)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	route.SortProbeResults(results)

	fmt.Fprintln(os.Stderr, "\nsummary (prefer top successes):")
	fmt.Printf("%-6s %-6s %8s  %s\n", "NAME", "RESULT", "TIME", "DETAIL")
	for _, r := range results {
		res, detail := "fail", r.Err
		if r.Skipped {
			res = "skip"
		} else if r.OK {
			res = "ok"
			detail = route.ShortLink(r.Link)
		}
		fmt.Printf("%-6s %-6s %8s  %s\n", r.Name, res, route.FormatLatency(r.Latency), detail)
	}

	for _, r := range results {
		if !r.OK {
			continue
		}
		info := route.FindBackend(r.Name)
		if info != nil && info.Status == "ok" {
			fmt.Fprintf(os.Stderr, "\nrecommended: uploader -b %s <file>\n", r.Name)
			return
		}
	}
	fmt.Fprintln(os.Stderr, "\nno working backend for this network")
	os.Exit(1)
}
