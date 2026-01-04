package wit_types

const (
	OptionNone = 0
	OptionSome = 1
)

type Option[T any] struct {
	tag   uint8
	value T
}

func (self Option[T]) Tag() uint8 {
	return self.tag
}

func (self Option[T]) Some() T {
	if self.tag != OptionSome {
		panic("tag mismatch")
	}
	return self.value
}

func (self Option[T]) SomeOr(value T) T {
	if self.tag != OptionSome {
		return value
	} else {
		return self.value
	}
}

func (self Option[T]) IsSome() bool {
	return self.tag == OptionSome
}

func (self Option[T]) IsNone() bool {
	return self.tag == OptionNone
}

func None[T any]() Option[T] {
	return Option[T]{OptionNone, make([]T, 1)[0]}
}

func Some[T any](value T) Option[T] {
	return Option[T]{OptionSome, value}
}
