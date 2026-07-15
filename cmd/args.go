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
