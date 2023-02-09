package main

import (
	. "wit_strings_go/gen"
)

func init() {
	n := StringsImpl{}
	SetStrings(n)
}

type StringsImpl struct{}

func (s StringsImpl) TestImports() {
	ImportsTakeBasic("latin utf16")
	if ImportsReturnUnicode() != "🚀🚀🚀 𠈄𓀀" {
		panic("ImportsReturnUnicode")
	}
}

func (s StringsImpl) ReturnEmpty() string {
	return ""
}

func (s StringsImpl) Roundtrip(a string) string {
	return a
}

func main() {}
