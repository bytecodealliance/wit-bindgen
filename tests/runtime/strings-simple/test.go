package export_cat

import "fmt"

func Foo(x string) {
	if x != "hello" {
		panic(fmt.Sprintf("unexpected value: `%v`", x))
	}
}

func Bar() string {
	return "world"
}
