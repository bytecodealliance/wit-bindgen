//@ wasmtime-flags = '-Wcomponent-model-implements'

package export_wit_world

import (
	"wit_component/backup"
	"wit_component/primary"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Run() {
	// Each labeled import is its own instance of `store`, so values set
	// through `primary` are independent from those set through `backup`.
	primary.Set("key", "from-primary")
	backup.Set("key", "from-backup")

	assertSome(primary.Get("key"), "from-primary")
	assertSome(backup.Get("key"), "from-backup")

	assertNone(primary.Get("missing"))
	assertNone(backup.Get("missing"))
}

func assertSome(opt Option[string], expected string) {
	if opt.Tag() != OptionSome || opt.Some() != expected {
		panic("unexpected value")
	}
}

func assertNone(opt Option[string]) {
	if opt.Tag() != OptionNone {
		panic("expected none")
	}
}
