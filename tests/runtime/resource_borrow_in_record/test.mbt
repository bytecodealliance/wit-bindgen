//@ [lang]
//@ path = 'gen/interface/test/resource-borrow-in-record/to-test/impl.mbt'

///|
struct Thing {
  contents : String
}

///|
pub fn Thing::thing(s : String) -> Thing {
  { contents: s + " new", }
}

///|
pub fn Thing::get(self : Thing) -> String {
  self.contents + " get"
}

///|
fn Thing::dtor(_self : Thing) -> Unit {
  ()
}

///|
pub fn test_(values : Array[Foo]) -> Array[ThingHandle] {
  values.map(fn(value) {
    ThingHandle::new({ contents: value.thing.contents + " test", })
  })
}
