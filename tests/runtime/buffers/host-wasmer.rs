wit_bindgen_wasmer::export!("./tests/runtime/buffers/imports.wit");

use anyhow::Result;
use imports::*;
use wasmer::WasmerEnv;
use wit_bindgen_wasmer::exports::{PullBuffer, PushBuffer};
use wit_bindgen_wasmer::Le;

#[derive(WasmerEnv, Clone)]
pub struct MyImports;

impl Imports for MyImports {
    fn buffer_u8(&mut self, in_: &[u8], out: &mut [u8]) -> u32 {
        assert_eq!(in_, [0]);
        assert_eq!(out.len(), 10);
        out[0] = 1;
        out[1] = 2;
        out[2] = 3;
        3
    }

    fn buffer_u32(&mut self, in_: &[Le<u32>], out: &mut [Le<u32>]) -> u32 {
        assert_eq!(in_.len(), 1);
        assert_eq!(in_[0].get(), 0);
        assert_eq!(out.len(), 10);
        out[0].set(1);
        out[1].set(2);
        out[2].set(3);
        3
    }

    fn buffer_bool(&mut self, in_: PullBuffer<'_, bool>, mut out: PushBuffer<'_, bool>) -> u32 {
        assert!(in_.len() <= out.capacity());
        let len = in_.len();
        for item in in_.iter() {
            let item = item.unwrap();
            out.write(Some(!item)).unwrap();
        }
        len as u32
    }

    // fn buffer_string(
    //     &mut self,
    //     in_: PullBuffer<'_, GuestPtr<'_, str>>,
    //     mut out: PushBuffer<'_, String>,
    // ) -> u32 {
    //     assert!(in_.len() < out.capacity());
    //     let len = in_.len();
    //     for item in in_.iter().unwrap() {
    //         let item = item.unwrap();
    //         let s = item.borrow().unwrap();
    //         out.write(Some(s.to_uppercase())).unwrap();
    //     }
    //     len as u32
    // }

    // fn buffer_list_bool(
    //     &mut self,
    //     in_: PullBuffer<'_, Vec<bool>>,
    //     mut out: PushBuffer<'_, Vec<bool>>,
    // ) -> u32 {
    //     assert!(in_.len() < out.capacity());
    //     let len = in_.len();
    //     for item in in_.iter().unwrap() {
    //         let item = item.unwrap();
    //         out.write(Some(item.into_iter().map(|b| !b).collect()))
    //             .unwrap();
    //     }
    //     len as u32
    // }

    // fn buffer_buffer_bool(&mut self, in_: PullBuffer<'_, PullBuffer<'_, bool>>) {
    //     assert_eq!(in_.len(), 1);
    //     let buf = in_.iter().unwrap().next().unwrap().unwrap();
    //     assert_eq!(buf.len(), 5);
    //     assert_eq!(
    //         buf.iter()
    //             .unwrap()
    //             .collect::<Result<Vec<bool>, _>>()
    //             .unwrap(),
    //         [true, false, true, true, false]
    //     );
    // }

    fn buffer_mutable1(&mut self, a: Vec<PullBuffer<'_, bool>>) {
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].len(), 5);
        assert_eq!(
            a[0].iter().collect::<Result<Vec<_>, _>>().unwrap(),
            [true, false, true, true, false]
        );
    }

    fn buffer_mutable2(&mut self, mut a: Vec<&mut [u8]>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].len() > 4);
        a[0][..4].copy_from_slice(&[1, 2, 3, 4]);
        return 4;
    }

    fn buffer_mutable3(&mut self, mut a: Vec<PushBuffer<'_, bool>>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].capacity() > 3);
        a[0].write([false, true, false].iter().copied()).unwrap();
        return 3;
    }

    fn buffer_in_record(&mut self, _: BufferInRecord<'_>) {}
    fn buffer_typedef(
        &mut self,
        _: ParamInBufferU8<'_>,
        _: ParamOutBufferU8<'_>,
        _: ParamInBufferBool<'_>,
        _: ParamOutBufferBool<'_>,
    ) {
    }
}

wit_bindgen_wasmer::import!("./tests/runtime/buffers/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let exports = crate::instantiate(
        wasm,
        |store, import_object| imports::add_to_imports(store, import_object, MyImports),
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    exports.test_imports()?;
    // fn buffers(wasm: &Wasm<Context>) -> Result<()> {
    //     let mut out = [0; 10];
    //     let n = wasm.buffer_u8(&[0u8], &mut out)? as usize;
    //     assert_eq!(n, 3);
    //     assert_eq!(&out[..n], [1, 2, 3]);
    //     assert!(out[n..].iter().all(|x| *x == 0));

    //     let mut out = [0; 10];
    //     let n = wasm.buffer_u32(&[0], &mut out)? as usize;
    //     assert_eq!(n, 3);
    //     assert_eq!(&out[..n], [1, 2, 3]);
    //     assert!(out[n..].iter().all(|x| *x == 0));

    //     assert_eq!(wasm.buffer_bool(&mut iter::empty(), &mut Vec::new())?, 0);
    //     assert_eq!(wasm.buffer_string(&mut iter::empty(), &mut Vec::new())?, 0);
    //     assert_eq!(
    //         wasm.buffer_list_bool(&mut iter::empty(), &mut Vec::new())?,
    //         0
    //     );

    //     let mut bools = [true, false, true].iter().copied();
    //     let mut out = Vec::with_capacity(4);
    //     let n = wasm.buffer_bool(&mut bools, &mut out)?;
    //     assert_eq!(n, 3);
    //     assert_eq!(out, [false, true, false]);

    //     let mut strings = ["foo", "bar", "baz"].iter().copied();
    //     let mut out = Vec::with_capacity(3);
    //     let n = wasm.buffer_string(&mut strings, &mut out)?;
    //     assert_eq!(n, 3);
    //     assert_eq!(out, ["FOO", "BAR", "BAZ"]);

    //     let a = &[true, false, true][..];
    //     let b = &[false, false][..];
    //     let list = [a, b];
    //     let mut lists = list.iter().copied();
    //     let mut out = Vec::with_capacity(4);
    //     let n = wasm.buffer_list_bool(&mut lists, &mut out)?;
    //     assert_eq!(n, 2);
    //     assert_eq!(out, [vec![false, true, false], vec![true, true]]);

    //     let a = [true, false, true, true, false];
    //     // let mut bools = a.iter().copied();
    //     // let mut list = [&mut bools as &mut dyn ExactSizeIterator<Item = _>];
    //     // let mut buffers = list.iter_mut().map(|b| &mut **b);
    //     // wasm.buffer_buffer_bool(&mut buffers)?;

    //     let mut bools = a.iter().copied();
    //     wasm.buffer_mutable1(&mut [&mut bools])?;

    //     let mut dst = [0; 10];
    //     let n = wasm.buffer_mutable2(&mut [&mut dst])? as usize;
    //     assert_eq!(n, 4);
    //     assert_eq!(&dst[..n], [1, 2, 3, 4]);

    //     let mut out = Vec::with_capacity(10);
    //     let n = wasm.buffer_mutable3(&mut [&mut out])?;
    //     assert_eq!(n, 3);
    //     assert_eq!(out, [false, true, false]);

    //     Ok(())
    // }

    Ok(())
}
