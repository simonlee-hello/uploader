package cmd

import "strings"

// reorderArgs moves flags before positional args so flag.FlagSet can parse
// commands like: uploader probe temp lit -timeout 20
func reorderArgs(args []string, valueFlags map[string]bool) []string {
	var flags, rest []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		if a == "--" {
			rest = append(rest, args[i+1:]...)
			break
		}
		if !strings.HasPrefix(a, "-") || a == "-" {
			rest = append(rest, a)
			continue
		}
		flags = append(flags, a)
		name := a
		if j := strings.IndexByte(a, '='); j >= 0 {
			continue
		}
		// -timeout 20 / -parallel 3
		if valueFlags[name] && i+1 < len(args) && !strings.HasPrefix(args[i+1], "-") {
			i++
			flags = append(flags, args[i])
		}
	}
	return append(flags, rest...)
}

// suggestFlag returns the closest known flag name for a typo, or "".
func suggestFlag(got string, known []string) string {
	got = strings.TrimLeft(got, "-")
	best, bestDist := "", 3 // only suggest if edit distance <= 2
	for _, k := range known {
		d := editDistance(got, k)
		if d < bestDist {
			bestDist = d
			best = k
		}
	}
	return best
}

func editDistance(a, b string) int {
	if a == b {
		return 0
	}
	la, lb := len(a), len(b)
	if la == 0 {
		return lb
	}
	if lb == 0 {
		return la
	}
	prev := make([]int, lb+1)
	cur := make([]int, lb+1)
	for j := 0; j <= lb; j++ {
		prev[j] = j
	}
	for i := 1; i <= la; i++ {
		cur[0] = i
		for j := 1; j <= lb; j++ {
			cost := 1
			if a[i-1] == b[j-1] {
				cost = 0
			}
			ins := cur[j-1] + 1
			del := prev[j] + 1
			sub := prev[j-1] + cost
			cur[j] = ins
			if del < cur[j] {
				cur[j] = del
			}
			if sub < cur[j] {
				cur[j] = sub
			}
		}
		prev, cur = cur, prev
	}
	return prev[lb]
}
