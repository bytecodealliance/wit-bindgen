package wit_types

const (
	OptionNone = 0
	OptionSome = 1
)

type Option[T any] struct {
	value *T
}

func (self Option[T]) Tag() uint8 {
	if self.value == nil {
		return OptionNone
	} else {
		return OptionSome
	}
}

func (self Option[T]) Some() T {
	if self.value == nil {
		panic("tag mismatch")
	}
	return *self.value
}

func (self Option[T]) SomeOr(value T) T {
	if self.value == nil {
		return value
	} else {
		return *self.value
	}
}

func None[T any]() Option[T] {
	return Option[T]{nil}
}

func Some[T any](value T) Option[T] {
	return Option[T]{&value}
}
