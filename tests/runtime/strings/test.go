package export_test_strings_to_test

import "fmt"

func TakeBasic(x string) {
	if x != "latin utf16" {
		panic(fmt.Sprintf("unexpected value: `%v`", x))
	}
}

func ReturnUnicode() string {
	return "🚀🚀🚀 𠈄𓀀"
}

func ReturnEmpty() string {
	return ""
}

func Roundtrip(x string) string {
	return x
}
