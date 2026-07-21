//@ [lang] 
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
pub async fn read_stream(
  x : @async-core.Stream[Byte],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  let chunk0 = x.read(1).unwrap()
  assert_eq(chunk0.length(), 1)
  assert_eq(chunk0[0], b'\x00')

  let chunk1 = x.read(2).unwrap()
  assert_eq(chunk1.length(), 2)
  assert_eq(chunk1[0], b'\x01')
  assert_eq(chunk1[1], b'\x02')

  let chunk2 = x.read(1).unwrap()
  assert_eq(chunk2.length(), 1)
  assert_eq(chunk2[0], b'\x03')

  let chunk3 = x.read(0x7fffffff).unwrap()
  assert_eq(chunk3.length(), 1)
  assert_eq(chunk3[0], b'\x04')

  x.drop()
}
