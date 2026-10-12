//@ [lang]
//@ path = 'gen/interface/test/managed-resources/api/impl.mbt'

///|
struct Counter {
  mut value : UInt
}

///|
let drops : Ref[UInt] = Ref(0)

///|
let saved_alias : Ref[CounterHandle?] = Ref(None)

///|
pub fn Counter::counter(value : UInt) -> Counter {
  { value, }
}

///|
pub fn Counter::get(self : Counter) -> UInt {
  self.value
}

///|
pub fn Counter::add(self : Counter, delta : UInt) -> Unit {
  self.value += delta
}

///|
fn Counter::dtor(self : Counter) -> Unit {
  drops.val += 1
  if self.value == 999 {
    // Destruction can reenter storage allocation and reuse the released slot.
    CounterHandle::counter(1).drop()
  }
}

///|
pub fn read(value : Counter) -> UInt {
  value.value
}

///|
pub fn read_record(value : Borrowed) -> UInt {
  value.value.value
}

///|
pub fn bounce(value : CounterHandle) -> CounterHandle {
  value.add(10)
  saved_alias.val = Some(value)
  value
}

///|
pub fn consume(value : CounterHandle) -> Unit {
  let other = value
  value.drop()
  other.drop()
}

///|
pub fn drop_saved_alias() -> Unit {
  if saved_alias.val is Some(value) {
    value.drop()
    saved_alias.val = None
  }
}

///|
pub fn drop_count() -> UInt {
  drops.val
}

///|
struct CheckedCounter {
  value : UInt
}

///|
let checked_drops : Ref[UInt] = Ref(0)

///|
pub fn CheckedCounter::checked_counter(
  value : UInt,
) -> Result[CheckedCounter, String] {
  if value == 0 {
    Err("zero")
  } else {
    Ok({ value, })
  }
}

///|
pub fn CheckedCounter::get(self : CheckedCounter) -> UInt {
  self.value
}

///|
fn CheckedCounter::dtor(_self : CheckedCounter) -> Unit {
  checked_drops.val += 1
}

///|
pub fn checked_drop_count() -> UInt {
  checked_drops.val
}
