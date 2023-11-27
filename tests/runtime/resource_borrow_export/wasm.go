package main

import (
	. "wit_resource_borrow_export_go/gen"
)

func init() {
	n := &Test{}
	SetExportsTestResourceBorrowExportTest(n)
}

type Test struct{}
type MyThing struct {
	val uint32
}

func (e Test) ConstructorThing(v uint32) ExportsTestResourceBorrowExportTestThing {
	return &MyThing{val: v + 1}
}

func (e Test) Foo(v ExportsTestResourceBorrowExportTestThing) uint32 {
	return v.(*MyThing).val + 2
}

func main() {}
