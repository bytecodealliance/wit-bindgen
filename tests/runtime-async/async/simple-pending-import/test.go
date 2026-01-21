package export_a_b_i

import "go.bytecodealliance.org/wit-bindgen/wit_async"

func F() {
	for i := 0; i < 10; i++ {
		wit_async.Yield()
	}
}
