//@ [lang]
//@ path = 'gen/interface/my/test_/leafInterface/stub.mbt'

///|
let leaf_values : Map[Int, String] = {}

///|
let next_leaf_rep : Ref[Int] = { val: 1 }

///| Destructor of the resource.
pub fn LeafThing::dtor(self : LeafThing) -> Unit {
  leaf_values.remove(self.rep())
}

///|
pub fn LeafThing::leaf_thing(s : String) -> LeafThing {
  let rep = next_leaf_rep.val
  next_leaf_rep.val += 1
  leaf_values[rep] = s
  LeafThing::new(rep)
}

///|
pub fn LeafThing::get(self : LeafThing) -> String {
  leaf_values[self.rep()]
}
