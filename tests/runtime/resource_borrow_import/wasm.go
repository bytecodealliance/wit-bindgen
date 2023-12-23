package main

import (
	. "wit_resource_borrow_import_go/gen"
)

func init() {
	n := &Import{}
	SetResourceBorrowImport(n)
}

type Import struct{}

func (e Import) Test(v uint32) uint32 {
	thing := NewThing(v + 1)
	defer thing.Drop()
	return TestResourceBorrowImportTestFoo(thing) + 4
	
}

func main() {}
