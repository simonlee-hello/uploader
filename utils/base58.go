package utils

// Base58Encode encodes src using Bitcoin-style base58 alphabet.
func Base58Encode(src []byte) string {
	const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
	if len(src) == 0 {
		return ""
	}
	zeros := 0
	for zeros < len(src) && src[zeros] == 0 {
		zeros++
	}
	size := len(src)*138/100 + 1
	buf := make([]byte, size)
	high := size - 1
	for _, b := range src[zeros:] {
		carry := int(b)
		for j := size - 1; j > high || carry != 0; j-- {
			carry += 256 * int(buf[j])
			buf[j] = byte(carry % 58)
			carry /= 58
			high = j
		}
	}
	for i := high; i < size && buf[i] == 0; i++ {
		high = i + 1
	}
	out := make([]byte, zeros+size-high)
	for i := 0; i < zeros; i++ {
		out[i] = alphabet[0]
	}
	for i, j := zeros, high; j < size; i, j = i+1, j+1 {
		out[i] = alphabet[buf[j]]
	}
	return string(out)
}
