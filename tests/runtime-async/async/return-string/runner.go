//@ wasmtime-flags = '-Wcomponent-model-async'

package export_wit_world

import (
	"fmt"
	"runtime"

	test "wit_component/my_test_i"
)

/*

This tests for pinner leaks in generated Go code for async exported
function that return heap-allocated types (strings, lists, etc.). Without
`pinner.Unpin()`, the `runtime.Pinner` object goes out of scope after
the function returns with pinned pointers still alive. When GC finalizes
the Pinner, it panics:

```
panic: runtime error: runtime.Pinner: found leaking pinned pointer;
forgot to call Unpin()?
```
*/

func Run() {
	// Perform a heap allocation
	got := test.ReturnString()
	if got != "hello" {
		panic(fmt.Sprintf("expected \"hello\", got %q", got))
	}

	// Force GC to finalize any leaked Pinners
	runtime.GC()
}
