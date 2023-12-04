package main

import (
	exports "smoke/exports"
	smoke "smoke/imports/test/smoke"
)

func init() {
	n := SmokeImpl{}
	exports.SetSmoke(n)
}

type SmokeImpl struct{}

func (s SmokeImpl) Thunk() {
	smoke.Thunk()
}

func main() {}
