package export_wit_world

import (
	. "wit_component/wit_world"
)

func ToplevelExport(a *Thing) *Thing {
	return ToplevelImport(a)
}
