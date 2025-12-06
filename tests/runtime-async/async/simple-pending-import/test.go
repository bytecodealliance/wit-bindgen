package export_a_b_i

import "wit_component/wit_async"

func F() {
	for i := 0; i < 10; i++ {
		wit_async.Yield()
	}
}
