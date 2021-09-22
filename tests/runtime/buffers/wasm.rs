witx_bindgen_rust::import!("./tests/runtime/buffers/imports.witx");
witx_bindgen_rust::export!("./tests/runtime/buffers/exports.witx");

use std::iter;

struct Exports;

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;
        use witx_bindgen_rust::imports::{PullBuffer, PushBuffer};

        let _guard = test_rust_wasm::guard();

        let mut out = [0; 10];
        let n = buffer_u8(&[0u8], &mut out) as usize;
        assert_eq!(n, 3);
        assert_eq!(&out[..n], [1, 2, 3]);
        assert!(out[n..].iter().all(|x| *x == 0));

        let mut out = [0; 10];
        let n = buffer_u32(&[0], &mut out) as usize;
        assert_eq!(n, 3);
        assert_eq!(&out[..n], [1, 2, 3]);
        assert!(out[n..].iter().all(|x| *x == 0));

        let mut space1 = [0; 200];
        let mut space2 = [0; 200];

        assert_eq!(
            buffer_bool(
                &mut PullBuffer::new(&mut space1, &mut iter::empty()),
                &mut PushBuffer::new(&mut space2)
            ),
            0
        );
        // assert_eq!(
        //     buffer_string(
        //         &mut PullBuffer::new(&mut space1, &mut iter::empty()),
        //         &mut PushBuffer::new(&mut space2)
        //     ),
        //     0
        // );
        // assert_eq!(
        //     buffer_list_bool(
        //         &mut PullBuffer::new(&mut space1, &mut iter::empty()),
        //         &mut PushBuffer::new(&mut space2)
        //     ),
        //     0
        // );

        let mut bools = [true, false, true].iter().copied();
        let mut out = PushBuffer::new(&mut space2);
        let n = buffer_bool(&mut PullBuffer::new(&mut space1, &mut bools), &mut out);
        unsafe {
            assert_eq!(n, 3);
            assert_eq!(out.into_iter(3).collect::<Vec<_>>(), [false, true, false]);
        }

        // let mut strings = ["foo", "bar", "baz"].iter().copied();
        // let mut out = PushBuffer::new(&mut space2);
        // let n = buffer_string(&mut PullBuffer::new(&mut space1, &mut strings), &mut out);
        // unsafe {
        //     assert_eq!(n, 3);
        //     assert_eq!(out.into_iter(3).collect::<Vec<_>>(), ["FOO", "BAR", "BAZ"]);
        // }

        // let a = &[true, false, true][..];
        // let b = &[false, false][..];
        // let list = [a, b];
        // let mut lists = list.iter().copied();
        // let mut out = PushBuffer::new(&mut space2);
        // let n = buffer_list_bool(&mut PullBuffer::new(&mut space1, &mut lists), &mut out);
        // unsafe {
        //     assert_eq!(n, 2);
        //     assert_eq!(
        //         out.into_iter(2).collect::<Vec<_>>(),
        //         [vec![false, true, false], vec![true, true]]
        //     );
        // }

        let a = [true, false, true, true, false];
        // let mut bools = a.iter().copied();
        // let mut b = PullBuffer::new(&mut space2, &mut bools);
        // let mut list = [&mut b];
        // let mut buffers = &mut list.iter_mut().map(|b| &mut **b);
        // buffer_buffer_bool(&mut PullBuffer::new(&mut space1, &mut buffers));

        let mut bools = a.iter().copied();
        buffer_mutable1(&mut [&mut PullBuffer::new(&mut space1, &mut bools)]);

        let n = buffer_mutable2(&mut [&mut space2]) as usize;
        assert_eq!(n, 4);
        assert_eq!(&space2[..n], [1, 2, 3, 4]);

        let mut out = PushBuffer::new(&mut space1);
        let n = buffer_mutable3(&mut [&mut out]);
        unsafe {
            assert_eq!(n, 3);
            assert_eq!(out.into_iter(3).collect::<Vec<_>>(), [false, true, false],);
        }
    }

    // fn buffer_u8( in_: InBufferRaw<'_, u8>, out: OutBufferRaw<'_, u8>) -> u32 {
    //     assert_eq!(in_.len(), 1);
    //     let mut input = [0];
    //     in_.copy(&mut input);
    //     assert_eq!(input, [0]);

    //     assert_eq!(out.capacity(), 10);
    //     out.write(&[1, 2, 3]);
    //     3
    // }

    // fn buffer_u32( in_: InBufferRaw<'_, u32>, out: OutBufferRaw<'_, u32>) -> u32 {
    //     assert_eq!(in_.len(), 1);
    //     let mut input = [0];
    //     in_.copy(&mut input);
    //     assert_eq!(input, [0]);

    //     assert_eq!(out.capacity(), 10);
    //     out.write(&[1, 2, 3]);
    //     3
    // }

    // fn buffer_bool( in_: InBuffer<'_, bool>, out: OutBuffer<'_, bool>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_.iter(&mut storage).map(|b| !b).collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // fn buffer_string( in_: InBuffer<'_, String>, out: OutBuffer<'_, String>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_
    //         .iter(&mut storage)
    //         .map(|s| s.to_uppercase())
    //         .collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // fn buffer_list_bool( in_: InBuffer<'_, Vec<bool>>, out: OutBuffer<'_, Vec<bool>>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_
    //         .iter(&mut storage)
    //         .map(|s| s.into_iter().map(|b| !b).collect::<Vec<_>>())
    //         .collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // // fn buffer_buffer_bool( in_: InBuffer<'_, InBuffer<'_, bool>>) {
    // //     assert_eq!(in_.len(), 1);
    // //     let mut storage = vec![0; in_.len() * in_.element_size()];
    // //     let buf = in_.iter(&mut storage).next().unwrap();
    // //     assert_eq!(buf.len(), 5);
    // //     let mut storage2 = vec![0; buf.len() * buf.element_size()];
    // //     assert_eq!(
    // //         buf.iter(&mut storage2).collect::<Vec<bool>>(),
    // //         [true, false, true, true, false]
    // //     );
    // // }

    // fn buffer_mutable1( a: Vec<InBuffer<'_, bool>>) {
    //     assert_eq!(a.len(), 1);
    //     assert_eq!(a[0].len(), 5);
    //     let mut storage = vec![0; a[0].len() * a[0].element_size()];
    //     assert_eq!(
    //         a[0].iter(&mut storage).collect::<Vec<_>>(),
    //         [true, false, true, true, false]
    //     );
    // }

    // fn buffer_mutable2( a: Vec<OutBufferRaw<'_, u8>>) -> u32 {
    //     assert_eq!(a.len(), 1);
    //     assert!(a[0].capacity() > 4);
    //     a[0].write(&[1, 2, 3, 4]);
    //     return 4;
    // }

    // fn buffer_mutable3( a: Vec<OutBuffer<'_, bool>>) -> u32 {
    //     assert_eq!(a.len(), 1);
    //     assert!(a[0].capacity() > 3);
    //     let mut storage = [0; 200];
    //     a[0].write(&mut storage, [false, true, false].iter().copied());
    //     return 3;
    // }

    // fn buffer_in_record( _: BufferInRecord<'_>) {}
    // fn buffer_typedef(
    //
    //     _: ParamInBufferU8<'_>,
    //     _: ParamOutBufferU8<'_>,
    //     _: ParamInBufferBool<'_>,
    //     _: ParamOutBufferBool<'_>,
    // ) {
    // }
}
