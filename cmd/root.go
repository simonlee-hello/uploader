package cmd

import (
	"bufio"
	"flag"
	"fmt"
	"os"

	"uploader/apis"
	fichier "uploader/apis/public/1fichier"
	"uploader/apis/public/gofile"
	"uploader/apis/public/wenshushu"
	"uploader/crypto"
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
)

func Execute() {
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
	fs.BoolVar(&flagEncrypt, "encrypt", false, "encrypt before upload")
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
	return fs
}

func applyGlobalConfig() {
	cfg := apis.TransferConfig()
	cfg.CryptoMode = flagEncrypt
	cfg.CryptoKey = flagEncryptKey
	cfg.NoBarMode = flagNoProgress || flagSilent
	apis.DebugMode = flagVerbose
	apis.MuteMode = flagSilent
	apis.Output = flagResult
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
		"-b": true, "-backend": true, "-encrypt-key": true,
		"-o": true, "-result": true, "-password": true, "-cookie": true,
		"-block": true, "-timeout": true, "-parallel": true,
		"-api-key": true, "-email": true,
	})
	fs := newUploadFlagSet("uploader")
	if err := fs.Parse(args); err != nil {
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
	files := uploadWalker(fs.Args())
	if flagBackend == "" {
		if len(files) > 0 {
			printBackendHint()
			os.Exit(1)
		}
		printHelp()
		return
	}
	info := findBackend(flagBackend)
	if info == nil {
		fmt.Fprintf(os.Stderr, "unknown backend %q\n", flagBackend)
		os.Exit(1)
	}
	if len(files) == 0 {
		fmt.Fprintf(os.Stderr, "usage: uploader -b %s <file>\n", info.Name)
		os.Exit(1)
	}
	applyBackendOptions(info.Name)
	apis.Upload(files, info.Backend)
}

func runBackends() {
	fmt.Print(formatBackendTable())
}

func runCrypto(encrypt bool, args []string) {
	fs := flag.NewFlagSet("crypto", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	crypto.RegisterFlags(fs)
	fs.BoolVar(&flagKeep, "keep", false, "wait for enter on exit")
	fs.BoolVar(&flagHelp, "h", false, "help")
	fs.BoolVar(&flagHelp, "help", false, "help")
	if err := fs.Parse(args); err != nil {
		os.Exit(2)
	}
	if flagHelp {
		if encrypt {
			fmt.Fprintln(os.Stderr, "usage: uploader encrypt [-key pass] [-output path] <file>")
		} else {
			fmt.Fprintln(os.Stderr, "usage: uploader decrypt -key pass [-output path] <file>")
		}
		return
	}
	files := uploadWalker(fs.Args())
	if len(files) == 0 {
		fmt.Fprintln(os.Stderr, "no file")
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
		}
	}
}

func printHelp() {
	fmt.Print(`uploader — multi-backend file uploader

Usage:
  uploader -b <backend> <file...>
  uploader backends
  uploader probe [backend...]
  uploader encrypt|decrypt [options] <file...>

Examples:
  uploader -b temp ./video.mkv
  uploader -b gof -s ./a.bin ./b.bin
  uploader probe
  uploader probe temp lit gof -timeout 20

Backends:
`)
	fmt.Print(formatBackendTable())
	fmt.Print(`
Flags:
  -b, -backend      backend name
  -encrypt          encrypt stream before upload
  -encrypt-key      encryption key
  -silent           print link only
  -no-progress      disable progress
  -o, -result       append links to file
  -v                verbose
  -password         share password (wss/fic)
  -s, -single       one link for many files (gof/wss)
  -ftp              1fichier FTP mode

Probe:
  -all              include disabled backends
  -parallel N       concurrency (default 3)
  -timeout SEC      per-backend timeout (default 45)
`)
}

func printBackendHint() {
	fmt.Fprintln(os.Stderr, "missing -b <backend>")
	fmt.Fprintln(os.Stderr, "example: uploader -b temp ./file")
	fmt.Fprintln(os.Stderr, "list: uploader backends | probe: uploader probe")
}
