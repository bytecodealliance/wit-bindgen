//@ [lang]
//@ path = 'gen/interface/exports/stub.mbt'
//@ pkg_config = '''
//@ import {
//@   "test/resources/interface/imports",
//@ }
//@
//@ warnings = "-44"
//@ '''

///|
struct X {
  mut value : Int
}

///|
struct Z {
  value : Int
}

///|
struct KebabCase {
  value : UInt
}

///|
let z_dropped : Ref[UInt] = Ref(1)

///|
fn X::dtor(_self : X) -> Unit {
  ()
}

///|
fn Z::dtor(_self : Z) -> Unit {
  z_dropped.val += 1
}

///|
fn KebabCase::dtor(_self : KebabCase) -> Unit {
  ()
}

///|
pub fn X::x(a : Int) -> X {
  { value: a, }
}

///|
pub fn X::get_a(self : X) -> Int {
  self.value
}

///|
pub fn X::set_a(self : X, a : Int) -> Unit {
  self.value = a
}

///|
pub fn X::add(self : XHandle, a : Int) -> XHandle {
  self.payload().value += a
  self
}

///|
pub fn Z::z(a : Int) -> Z {
  { value: a, }
}

///|
pub fn Z::get_a(self : Z) -> Int {
  self.value
}

///|
pub fn Z::num_dropped() -> UInt {
  z_dropped.val
}

///|
pub fn add(a : Z, b : Z) -> ZHandle {
  ZHandle::new({ value: a.value + b.value, })
}

///|
pub fn consume(x : XHandle) -> Unit {
  x.drop()
}

///|
pub fn KebabCase::kebab_case(a : UInt) -> KebabCase {
  { value: a, }
}

///|
pub fn KebabCase::get_a(self : KebabCase) -> UInt {
  self.value
}

///|
pub fn KebabCase::take_owned(self : KebabCaseHandle) -> UInt {
  defer self.drop()
  self.payload().value
}

///|
pub fn test_imports() -> Result[Unit, String] {
  let y = @imports.Y::y(10)
  defer y.drop()
  guard! y.get_a() == 10
  y.set_a(20)
  guard! y.get_a() == 20
  let y2 = @imports.Y::add(y, 20)
  guard! y2.get_a() == 40

  // test multiple instances
  let y1 = @imports.Y::y(1)
  defer y1.drop()
  let y2 = @imports.Y::y(2)
  defer y2.drop()
  guard! y1.get_a() == 1
  guard! y2.get_a() == 2
  y1.set_a(10)
  y2.set_a(20)
  guard! y1.get_a() == 10
  guard! y2.get_a() == 20
  let y3 = y1.add(20)
  let y4 = y2.add(30)
  guard! y3.get_a() == 30
  guard! y4.get_a() == 50
  Ok(())
}
