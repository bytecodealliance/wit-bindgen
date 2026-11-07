package export_my_test_i

import (
	"wit_component/my_test_i"

	witTypes "go.bytecodealliance.org/pkg/wit/types"
)

var writer *witTypes.StreamWriter[witTypes.Unit]

func PendingFuture(x *witTypes.FutureReader[witTypes.Unit]) {
	x.Read()
}

func PendingStream(x *witTypes.StreamReader[witTypes.Unit]) {
	x.Read(make([]witTypes.Unit, 1))
}

func MakeStream() *witTypes.StreamReader[witTypes.Unit] {
	tx, rx := my_test_i.MakeStreamUnit()
	writer = tx
	return rx
}

func PendingStreamWrite() {
	writer.Write(make([]witTypes.Unit, 1))
}
