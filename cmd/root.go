package cmd

import (
	"bufio"
	"flag"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"uploader/apis"
	"uploader/apis/methods"
	fichier "uploader/apis/public/1fichier"
	"uploader/apis/public/gofile"
	"uploader/apis/public/wenshushu"
	"uploader/crypto"
	"uploader/route"
)

var (
	flagBackend    string
	flagVersion    bool
	flagKeep       bool
	flagHelp       bool
	flagEncrypt    bool
	flagEncryptKey string
	flagNoProgress bool
	flagSilent     bool
	flagVerbose    bool
	flagResult     string

	flagPassword   string
	flagSingle     bool
	flagCookie     string
	flagBlock      int
	flagReqTimeout int
	flagParallel   int
	flagAPIKey     string
	flagEmail      string
	flagFTP        bool
	flagRecursive  bool
	flagForce      bool
	flagQuiet      bool
	flagAuto       bool
	flagHTTPTimeout int
)

func Execute() {
	route.SetupBackend = applyBackendOptions

	defer func() {
		if flagKeep {
			fmt.Fprint(os.Stderr, "press enter to exit...")
			_, _ = bufio.NewReader(os.Stdin).ReadString('\n')
		}
	}()

	if len(os.Args) < 2 {
		printHelp()
		return
	}

	switch os.Args[1] {
	case "backends", "list", "ls":
		runBackends()
	case "probe", "ping", "check":
		runProbe(os.Args[2:])
	case "encrypt":
		runCrypto(true, os.Args[2:])
	case "decrypt":
		runCrypto(false, os.Args[2:])
	case "-h", "-help", "--help", "help":
		printHelp()
	default:
		runUpload(os.Args[1:])
	}
}

func newUploadFlagSet(name string) *flag.FlagSet {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	fs.StringVar(&flagBackend, "b", "", "backend")
	fs.StringVar(&flagBackend, "backend", "", "backend")
	fs.BoolVar(&flagVersion, "version", false, "print version")
	fs.BoolVar(&flagKeep, "keep", false, "wait for enter on exit")
	fs.BoolVar(&flagHelp, "h", false, "help")
	fs.BoolVar(&flagHelp, "help", false, "help")
	fs.BoolVar(&flagEncrypt, "e", false, "encrypt before upload")
	fs.BoolVar(&flagEncrypt, "encrypt", false, "encrypt before upload")
	fs.StringVar(&flagEncryptKey, "k", "", "encryption key")
	fs.StringVar(&flagEncryptKey, "key", "", "encryption key")
	fs.StringVar(&flagEncryptKey, "encrypt-key", "", "encryption key")
	fs.BoolVar(&flagNoProgress, "no-progress", false, "disable progress")
	fs.BoolVar(&flagSilent, "silent", false, "print link only")
	fs.BoolVar(&flagVerbose, "v", false, "verbose")
	fs.BoolVar(&flagVerbose, "verbose", false, "verbose")
	fs.StringVar(&flagResult, "o", "", "append links to file")
	fs.StringVar(&flagResult, "result", "", "append links to file")
	fs.StringVar(&flagPassword, "password", "", "share password (wss/fic)")
	fs.BoolVar(&flagSingle, "s", false, "single link for multiple files")
	fs.BoolVar(&flagSingle, "single", false, "single link for multiple files")
	fs.StringVar(&flagCookie, "cookie", "", "wss cookie")
	fs.IntVar(&flagBlock, "block", 1048576, "wss block size")
	fs.IntVar(&flagReqTimeout, "timeout", 10, "wss request timeout sec")
	fs.IntVar(&flagParallel, "parallel", 2, "wss parallel uploads")
	fs.StringVar(&flagAPIKey, "api-key", "", "1fichier api key")
	fs.StringVar(&flagEmail, "email", "", "1fichier notify email")
	fs.BoolVar(&flagFTP, "ftp", false, "1fichier ftp upload")
	fs.BoolVar(&flagRecursive, "r", false, "upload each file under directory (no zip)")
	fs.BoolVar(&flagRecursive, "recursive", false, "upload each file under directory (no zip)")
	fs.BoolVar(&flagForce, "force", false, "allow flaky/down backends")
	fs.BoolVar(&flagQuiet, "q", false, "quiet: stdout links only, no stderr info")
	fs.BoolVar(&flagQuiet, "quiet", false, "quiet: stdout links only, no stderr info")
	fs.BoolVar(&flagAuto, "auto", false, "with -b: probe+failover (no -b already means auto)")
	fs.IntVar(&flagHTTPTimeout, "http-timeout", 600, "HTTP timeout seconds (0=no limit)")
	return fs
}

func applyGlobalConfig() {
	cfg := apis.TransferConfig()
	cfg.CryptoMode = flagEncrypt
	cfg.CryptoKey = flagEncryptKey
	cfg.NoBarMode = flagNoProgress || flagSilent || flagQuiet
	cfg.RecursiveDirs = flagRecursive
	apis.DebugMode = flagVerbose && !flagQuiet
	apis.MuteMode = flagSilent || flagQuiet
	apis.QuietMode = flagQuiet
	apis.Output = flagResult
	if flagHTTPTimeout > 0 {
		methods.HTTPTimeout = time.Duration(flagHTTPTimeout) * time.Second
	} else {
		methods.HTTPTimeout = 0
	}
}

func applyBackendOptions(name string) {
	block, timeout, parallel := flagBlock, flagReqTimeout, flagParallel
	if block <= 0 {
		block = 1048576
	}
	if timeout <= 0 {
		timeout = 10
	}
	if parallel <= 0 {
		parallel = 2
	}
	switch name {
	case "fic", "1fichier":
		fichier.Backend.SetPassword(flagPassword)
		fichier.Backend.SetAPIKey(flagAPIKey)
		fichier.Backend.SetEmail(flagEmail)
		fichier.Backend.SetFTP(flagFTP)
	case "gof", "gofile":
		gofile.Backend.Config.SingleMode = flagSingle
	case "wss", "wenshushu":
		wenshushu.Backend.Config.PassCode = flagPassword
		wenshushu.Backend.Config.SingleMode = flagSingle
		wenshushu.Backend.Config.Token = flagCookie
		wenshushu.Backend.Config.BlockSize = block
		wenshushu.Backend.Config.Interval = timeout
		wenshushu.Backend.Config.Parallel = parallel
	}
}

func runUpload(args []string) {
	args = reorderArgs(args, map[string]bool{
		"-b": true, "-backend": true,
		"-k": true, "-key": true, "-encrypt-key": true,
		"-o": true, "-result": true, "-password": true, "-cookie": true,
		"-block": true, "-timeout": true, "-parallel": true,
		"-api-key": true, "-email": true, "-http-timeout": true,
	})
	fs := newUploadFlagSet("uploader")
	fs.SetOutput(io.Discard)
	fs.Usage = func() {}
	if err := fs.Parse(args); err != nil {
		msg := err.Error()
		fmt.Fprintf(os.Stderr, "error: %s\n", msg)
		if strings.HasPrefix(msg, "flag provided but not defined: -") {
			unknown := strings.TrimPrefix(msg, "flag provided but not defined: -")
			if sug := suggestFlag(unknown, []string{
				"b", "backend", "e", "encrypt", "k", "key", "encrypt-key",
				"o", "result", "silent", "no-progress", "v", "verbose",
				"password", "s", "single", "ftp", "r", "recursive",
				"force", "q", "quiet", "auto", "http-timeout", "h", "help",
			}); sug != "" {
				fmt.Fprintf(os.Stderr, "did you mean -%s?\n", sug)
			}
		}
		fmt.Fprintln(os.Stderr, "usage: uploader [-b backend] [-e] [-k pass] <file...>")
		os.Exit(2)
	}
	if flagHelp {
		printHelp()
		return
	}
	if flagVersion {
		fmt.Println("uploader 1.0.0")
		return
	}
	applyGlobalConfig()
	files, err := uploadWalker(fs.Args())
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	// No -b → auto failover. With -b → pin that backend (unless -auto).
	primary := strings.TrimSpace(flagBackend)
	auto := primary == "" || flagAuto
	if primary == "" {
		primary = resolveDefaultBackend()
	}
	if len(files) == 0 {
		if primary != "" {
			fmt.Fprintf(os.Stderr, "usage: uploader -b %s <file>\n", primary)
			os.Exit(1)
		}
		printHelp()
		return
	}
	if err := uploadWithOptions(files, primary, auto, flagForce); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func runBackends() {
	fmt.Print(formatBackendTable())
}

func runCrypto(encrypt bool, args []string) {
	args = reorderArgs(args, map[string]bool{
		"-key": true, "-k": true, "-encrypt-key": true,
		"-output": true, "-o": true, "-out": true,
	})

	fs := flag.NewFlagSet("crypto", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	fs.Usage = func() {}
	crypto.RegisterFlags(fs)
	fs.BoolVar(&flagKeep, "keep", false, "wait for enter on exit")
	fs.BoolVar(&flagHelp, "h", false, "help")
	fs.BoolVar(&flagHelp, "help", false, "help")

	if err := fs.Parse(args); err != nil {
		msg := err.Error()
		fmt.Fprintf(os.Stderr, "error: %s\n", msg)
		if strings.HasPrefix(msg, "flag provided but not defined: -") {
			unknown := strings.TrimPrefix(msg, "flag provided but not defined: -")
			if sug := suggestFlag(unknown, []string{
				"k", "key", "encrypt-key", "o", "output", "out", "f", "force", "h", "help", "keep", "no-progress",
			}); sug != "" {
				fmt.Fprintf(os.Stderr, "did you mean -%s?\n", sug)
			}
		}
		printCryptoUsage(encrypt)
		os.Exit(2)
	}
	if flagHelp {
		printCryptoUsage(encrypt)
		return
	}
	files, err := uploadWalker(fs.Args())
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if len(files) == 0 {
		fmt.Fprintln(os.Stderr, "no file")
		printCryptoUsage(encrypt)
		os.Exit(1)
	}
	for _, f := range files {
		var err error
		if encrypt {
			err = crypto.Encrypt(f)
		} else {
			err = crypto.Decrypt(f)
		}
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	}
}

func printCryptoUsage(encrypt bool) {
	if encrypt {
		fmt.Fprintln(os.Stderr, "usage: uploader encrypt [-k pass] [-o path] [-force] <file>")
	} else {
		fmt.Fprintln(os.Stderr, "usage: uploader decrypt -k pass [-o path] [-force] <file>")
	}
	fmt.Fprintln(os.Stderr, "flags: -k/-key/-encrypt-key  -o/-output/-out  -f/-force")
}

func printHelp() {
	fmt.Print(`uploader — multi-backend file uploader

Usage:
  uploader <file...>                 # auto: probe then upload via fastest fitting backend
  uploader -b <backend> <file...>    # pin one backend
  uploader backends
  uploader probe [backend...]
  uploader encrypt|decrypt [options] <file...>

Examples:
  uploader ./video.mkv               # probe + pick fastest
  uploader -q ./file.bin             # quiet + auto
  uploader -b temp ./video.mkv
  uploader -b lit ./mydir            # zip directory then upload
  uploader -b lit -r ./mydir         # upload each file under mydir
  uploader -b lit -e -k pass ./file
  uploader -b gof -s ./a.bin ./b.bin
  uploader probe
  uploader probe temp lit gof -timeout 20
  uploader encrypt -k pass ./file
  uploader decrypt -k pass -o out.bin ./file.encrypt

Backends:
`)
	fmt.Print(formatBackendTable())
	fmt.Print(`
Flags:
  -b, -backend      pin backend (omit = auto: probe then upload)
  -e, -encrypt      encrypt stream before upload
  -k, -key, -encrypt-key  encryption key (upload)
  -r, -recursive    upload each file under a directory (default: zip dir)
  -q, -quiet        headless: links on stdout only, errors on stderr
  -auto             with -b: still probe+failover (omit -b already means auto)
  -force            allow flaky/down backends
  -http-timeout SEC global HTTP timeout (default 600, 0=none)
  -silent           print link only (alias of quiet progress)
  -no-progress      disable progress
  -o, -result       append links to file (upload)
  -v                verbose
  -password         share password (wss/fic)
  -s, -single       one link for many files (gof/wss)
  -ftp              1fichier FTP mode

Headless deploy:
  uploader -q ./file                # quiet + auto probe
  set UPLOADER_BACKEND=lit          # preferred when pinning / last-backend seed
  config: ~/.config/uploader/config # backend=lit
  avoid -keep in scripts (waits for Enter)

Encrypt/decrypt:
  -k, -key, -encrypt-key  password
  -o, -output, -out output path
  -f, -force        overwrite output

Probe:
  -all              include flaky and down backends
  -parallel N       concurrency (default 3)
  -timeout SEC      per-backend timeout (default 45)
`)
}

func printBackendHint() {
	fmt.Fprintln(os.Stderr, "example: uploader ./file  |  uploader -b temp ./file")
	fmt.Fprintln(os.Stderr, "list: uploader backends | probe: uploader probe")
}
