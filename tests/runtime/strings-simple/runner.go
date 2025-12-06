package export_wit_world

import (
	"fmt"
	test "wit_component/cat"
)

func Run() {
	test.Foo("hello")
	value := test.Bar()
	if value != "world" {
		panic(fmt.Sprintf("expected `world`; got `%v`", value))
	}
}
