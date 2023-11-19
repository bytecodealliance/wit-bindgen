package main

import (
	. "wit_resource_import_and_export_go/gen"
)

func init() {
	n := &MyTest{}
	t := &MyImportExport{}
	SetExportsTestResourceImportAndExportTest(t)
	SetResourceImportAndExport(n)
}

type MyTest struct {}

type MyImportExport struct {}
type MyThing struct {
	hostThing TestResourceImportAndExportTestThing
}

func (t MyImportExport) ConstructorThing(v uint32) ExportsTestResourceImportAndExportTestThing  {
	thing := &MyThing{
		hostThing: NewThing(v + 1),
	}
	return thing
}

func (t MyImportExport) StaticThingBaz(a ExportsTestResourceImportAndExportTestThing, b ExportsTestResourceImportAndExportTestThing) ExportsTestResourceImportAndExportTestThing {
	result := StaticThingBaz(a.(*MyThing).hostThing, b.(*MyThing).hostThing).Foo() + 4
	return t.ConstructorThing(result)
}

func (t *MyThing) MethodThingFoo() uint32 {
	return t.hostThing.Foo() + 2
}

func (t *MyThing) MethodThingBar(v uint32) {
	t.hostThing.Bar(v + 3)
} 

func (e MyTest) ToplevelExport(a ResourceImportAndExportThing) ResourceImportAndExportThing {
	return ResourceImportAndExportToplevelImport(a)
}

func main() {}