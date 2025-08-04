package methods

type TransferConfig struct {
	Parallel   int
	DebugMode  *bool
	NoBarMode  bool
	CryptoMode bool
	CryptoKey  string
}
