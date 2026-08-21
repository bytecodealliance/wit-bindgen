package export_wit_world

import (
	"fmt"
	test "wit_component/test_strings_to_test"
)

func Run() {
	test.TakeBasic("latin utf16")
	assertEqual(test.ReturnUnicode(), "🚀🚀🚀 𠈄𓀀")
	assertEqual(test.ReturnEmpty(), "")
	assertEqual(test.Roundtrip("🚀🚀🚀 𠈄𓀀"), "🚀🚀🚀 𠈄𓀀")
}

func assertEqual(a string, b string) {
	if a != b {
		panic(fmt.Sprintf("`%v` not equal to `%v`", a, b))
	}
}
