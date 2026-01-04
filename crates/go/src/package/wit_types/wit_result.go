package wit_types

const (
	ResultOk  = 0
	ResultErr = 1
)

type Result[T any, U any] struct {
	tag   uint8
	value any
}

func (self Result[T, U]) Tag() uint8 {
	return self.tag
}

func (self Result[T, U]) Ok() T {
	if self.tag != ResultOk {
		panic("tag mismatch")
	}
	return self.value.(T)
}

func (self Result[T, U]) Err() U {
	if self.tag != ResultErr {
		panic("tag mismatch")
	}
	return self.value.(U)
}

func (self Result[T, U]) IsErr() bool {
	return self.tag == ResultErr
}

func (self Result[T, U]) IsOk() bool {
	return self.tag == ResultOk
}

func Ok[T any, U any](value T) Result[T, U] {
	return Result[T, U]{ResultOk, value}
}

func Err[T any, U any](value U) Result[T, U] {
	return Result[T, U]{ResultErr, value}
}
