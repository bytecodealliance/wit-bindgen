package export_wit_world

import test "wit_component/test_resource_import_and_export_test"

func ToplevelExport(a *test.Thing) *test.Thing {
	return a
}
