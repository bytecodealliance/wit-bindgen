package main

import (
  . "wit_test_go/bindings"
)

func init() {
  n := TestImpl {}
  SetExportsABTheTest(n)
}

type TestImpl struct {}

func (s TestImpl) X() {
}

func main() {}
