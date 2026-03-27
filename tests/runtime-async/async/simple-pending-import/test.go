package export_a_b_i

import (
	witAsync "go.bytecodealliance.org/pkg/wit/async"
)

func F() {
	for i := 0; i < 10; i++ {
		witAsync.Yield()
	}
}
