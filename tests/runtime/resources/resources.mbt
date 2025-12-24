//@ [lang]
//@ path = 'gen/interface/exports/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["test/resources/interface/imports"] }"""

///|
let x : Map[Int, Int] = {}

///|
let x_counter : Ref[Int] = { val: 1 }

///|
let z : Map[Int, Int] = {}

///|
let z_counter : Ref[Int] = { val: 1 }

///|
let z_dropped : Ref[UInt] = { val: 1 }

///|
let k : Map[Int, UInt] = {}

///|
let k_counter : Ref[Int] = { val: 1 }

///|
/// Destructor of the resource.
pub fn X::dtor(self : X) -> Unit {
  x.remove(self.0)
}

///|
/// Destructor of the resource.
pub fn Z::dtor(self : Z) -> Unit {
  z_dropped.val += 1
  z.remove(self.0)
}

///|
/// Destructor of the resource.
pub fn KebabCase::dtor(self : KebabCase) -> Unit {
  k.remove(self.rep())
}

///|
pub fn X::x(a : Int) -> X {
  let rep = x_counter.val
  x_counter.val += 1
  x[rep] = a
  X::new(rep)
}

///|
pub fn X::get_a(self : X) -> Int {
  x[self.0]
}

///|
pub fn X::set_a(self : X, a : Int) -> Unit {
  x[self.0] = a
}

///|
pub fn X::add(self : X, a : Int) -> X {
  x[self.rep()] += a
  self
}

///|
pub fn Z::z(a : Int) -> Z {
  let rep = z_counter.val
  z_counter.val += 1
  z[rep] = a
  Z::new(rep)
}

///|
pub fn Z::get_a(self : Z) -> Int {
  z[self.0]
}

///|
pub fn Z::num_dropped() -> UInt {
  z_dropped.val
}

///|
pub fn add(a : Z, b : Z) -> Z {
  let rep = z_counter.val
  z_counter.val += 1
  z[rep] = z[a.0] + z[b.0]
  Z::new(rep)
}

///|
pub fn consume(x : X) -> Unit {
  x.drop()
}

///|
pub fn KebabCase::kebab_case(a : UInt) -> KebabCase {
  let rep = k_counter.val
  k_counter.val += 1
  k[rep] = a
  KebabCase::new(rep)
}

///|
pub fn KebabCase::get_a(self : KebabCase) -> UInt {
  k[self.rep()]
}

///|
pub fn KebabCase::take_owned(self : KebabCase) -> UInt {
  defer self.drop()
  k[self.rep()]
}

///|
pub fn test_imports() -> Result[Unit, String] {
  let y = @imports.Y::y(10)
  defer y.drop()
  guard y.get_a() == 10
  y.set_a(20)
  guard y.get_a() == 20
  let y2 = @imports.Y::add(y, 20)
  guard y2.get_a() == 40

  // test multiple instances
  let y1 = @imports.Y::y(1)
  defer y1.drop()
  let y2 = @imports.Y::y(2)
  defer y2.drop()
  guard y1.get_a() == 1
  guard y2.get_a() == 2
  y1.set_a(10)
  y2.set_a(20)
  guard y1.get_a() == 10
  guard y2.get_a() == 20
  let y3 = y1.add(20)
  let y4 = y2.add(30)
  guard y3.get_a() == 30
  guard y4.get_a() == 50
  Ok(())
}
