package export_test_numbers_numbers

func RoundtripU8(v uint8) uint8 {
	return v
}

func RoundtripS8(v int8) int8 {
	return v
}

func RoundtripU16(v uint16) uint16 {
	return v
}

func RoundtripS16(v int16) int16 {
	return v
}

func RoundtripU32(v uint32) uint32 {
	return v
}

func RoundtripS32(v int32) int32 {
	return v
}

func RoundtripU64(v uint64) uint64 {
	return v
}

func RoundtripS64(v int64) int64 {
	return v
}

func RoundtripF32(v float32) float32 {
	return v
}

func RoundtripF64(v float64) float64 {
	return v
}

func RoundtripChar(v rune) rune {
	return v
}

var scalar uint32 = 0

func SetScalar(v uint32) {
	scalar = v
}

func GetScalar() uint32 {
	return scalar
}
