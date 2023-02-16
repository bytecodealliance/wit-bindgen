package main

import (
	. "wit_smoke_go/gen"
)

func init() {
	n := SmokeImpl{}
	SetSmoke(n)
}

type SmokeImpl struct{}

func (s SmokeImpl) Thunk() {
	ImportsThunk()
}

func main() {}
