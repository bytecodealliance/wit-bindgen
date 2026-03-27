pub(crate) const EXTEND16: &str = r#"
///|
extern "wasm" fn mbt_ffi_extend16(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend16_s)
"#;

pub(crate) const EXTEND8: &str = r#"
///|
extern "wasm" fn mbt_ffi_extend8(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend8_s)
"#;

pub(crate) const STORE8: &str = r#"
///|
extern "wasm" fn mbt_ffi_store8(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store8)
"#;

pub(crate) const LOAD8_U: &str = r#"
///|
extern "wasm" fn mbt_ffi_load8_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_u)
"#;

pub(crate) const LOAD8: &str = r#"
///|
extern "wasm" fn mbt_ffi_load8(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_s)
"#;

pub(crate) const STORE16: &str = r#"
///|
extern "wasm" fn mbt_ffi_store16(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store16)
"#;

pub(crate) const LOAD16: &str = r#"
///|
extern "wasm" fn mbt_ffi_load16(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_s)
"#;

pub(crate) const LOAD16_U: &str = r#"
///|
extern "wasm" fn mbt_ffi_load16_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_u)
"#;

pub(crate) const STORE32: &str = r#"
///|
extern "wasm" fn mbt_ffi_store32(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store)
"#;

pub(crate) const LOAD32: &str = r#"
///|
extern "wasm" fn mbt_ffi_load32(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load)
"#;

pub(crate) const STORE64: &str = r#"
///|
extern "wasm" fn mbt_ffi_store64(offset : Int, value : Int64) =
  #|(func (param i32) (param i64) local.get 0 local.get 1 i64.store)
"#;

pub(crate) const LOAD64: &str = r#"
///|
extern "wasm" fn mbt_ffi_load64(offset : Int) -> Int64 =
  #|(func (param i32) (result i64) local.get 0 i64.load)
"#;

pub(crate) const STOREF32: &str = r#"
///|
extern "wasm" fn mbt_ffi_storef32(offset : Int, value : Float) =
  #|(func (param i32) (param f32) local.get 0 local.get 1 f32.store)
"#;

pub(crate) const LOADF32: &str = r#"
///|
extern "wasm" fn mbt_ffi_loadf32(offset : Int) -> Float =
  #|(func (param i32) (result f32) local.get 0 f32.load)
"#;

pub(crate) const STOREF64: &str = r#"
///|
extern "wasm" fn mbt_ffi_storef64(offset : Int, value : Double) =
  #|(func (param i32) (param f64) local.get 0 local.get 1 f64.store)
"#;

pub(crate) const LOADF64: &str = r#"
///|
extern "wasm" fn mbt_ffi_loadf64(offset : Int) -> Double =
  #|(func (param i32) (result f64) local.get 0 f64.load)
"#;

pub(crate) const STR2PTR: &str = r#"
///|
#owned(str)
extern "wasm" fn mbt_ffi_str2ptr(str : String) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const PTR2STR: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2str(ptr : Int, len : Int) -> String =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array16
  #| local.get 2)
"#;

pub(crate) const BYTES2PTR: &str = r#"
///|
#owned(bytes)
extern "wasm" fn mbt_ffi_bytes2ptr(bytes : FixedArray[Byte]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const PTR2BYTES: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2bytes(ptr : Int, len : Int) -> FixedArray[Byte] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array8
  #| local.get 2)
"#;

pub(crate) const UINT_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_uint_array2ptr(array : FixedArray[UInt]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const UINT64_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_uint64_array2ptr(array : FixedArray[UInt64]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const INT_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_int_array2ptr(array : FixedArray[Int]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const INT64_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_int64_array2ptr(array : FixedArray[Int64]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const FLOAT_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_float_array2ptr(array : FixedArray[Float]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const DOUBLE_ARRAY2PTR: &str = r#"
///|
#owned(array)
extern "wasm" fn mbt_ffi_double_array2ptr(array : FixedArray[Double]) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.const 8 i32.add)
"#;

pub(crate) const PTR2UINT_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2uint_array(ptr : Int, len : Int) -> FixedArray[UInt] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array32
  #| local.get 2)
"#;

pub(crate) const PTR2INT_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2int_array(ptr : Int, len : Int) -> FixedArray[Int] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array32
  #| local.get 2)
"#;

pub(crate) const PTR2FLOAT_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2float_array(ptr : Int, len : Int) -> FixedArray[Float] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array32
  #| local.get 2)
"#;

pub(crate) const PTR2UINT64_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2uint64_array(
  ptr : Int,
  len : Int,
) -> FixedArray[UInt64] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array64
  #| local.get 2)
"#;

pub(crate) const PTR2INT64_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2int64_array(ptr : Int, len : Int) -> FixedArray[Int64] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array64
  #| local.get 2)
"#;

pub(crate) const PTR2DOUBLE_ARRAY: &str = r#"
///|
extern "wasm" fn mbt_ffi_ptr2double_array(
  ptr : Int,
  len : Int,
) -> FixedArray[Double] =
  #|(func (param i32) (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 8 i32.sub local.tee 2
  #| local.get 1 call $moonbit.init_array64
  #| local.get 2)
"#;

pub(crate) const MALLOC: &str = r#"
///|
extern "wasm" fn mbt_ffi_malloc(size : Int) -> Int =
  #|(func (param i32) (result i32) (local i32)
  #| local.get 0 i32.const 4 i32.add call $moonbit.gc.malloc
  #| local.tee 1 i32.const 0 call $moonbit.init_array8
  #| local.get 1 i32.const 8 i32.add)
"#;

pub(crate) const FREE: &str = r#"
///|
extern "wasm" fn mbt_ffi_free(position : Int) =
  #|(func (param i32) local.get 0 i32.const 8 i32.sub call $moonbit.decref)
"#;

pub(crate) const CABI_REALLOC: &str = r#"
///|
pub fn mbt_ffi_cabi_realloc(
  src_offset : Int,
  src_size : Int,
  _dst_alignment : Int,
  dst_size : Int,
) -> Int {
  // malloc
  if src_offset == 0 && src_size == 0 {
    return mbt_ffi_malloc(dst_size)
  }
  // free
  if dst_size == 0 {
    mbt_ffi_free(src_offset)
    return 0
  }
  // realloc
  let dst = mbt_ffi_malloc(dst_size)
  mbt_ffi_copy(dst, src_offset, if src_size < dst_size { src_size } else { dst_size })
  mbt_ffi_free(src_offset)
  dst
}

///|
extern "wasm" fn mbt_ffi_copy(dest : Int, src : Int, len : Int) =
  #|(func (param i32) (param i32) (param i32) local.get 0 local.get 1 local.get 2 memory.copy)
"#;
