package main

import (
	. "wit_resource_borrow_simple_go/gen"
)

func init() {
	n := &Simple{}
	SetResourceBorrowSimple(n)
}

type Simple struct {}

func (e Simple) TestImports() {
	r := NewR()
	ResourceBorrowSimpleTest(r)
	r.Drop()
}

func main() {}