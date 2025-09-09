//@ [lang]
//@ path = 'gen/interface/a/b/i/stub.mbt'


pub async fn async_one_argument(_task: @ffi.WaitableTask, x : UInt) -> Unit raise {
    assert_eq(x,1)
}

pub async fn async_one_result(_task: @ffi.WaitableTask, ) -> UInt noraise {
    2
}

pub async fn async_one_argument_and_result(_task: @ffi.WaitableTask, x : UInt) -> UInt raise {
    assert_eq(x,3);
    4
}

pub async fn async_two_arguments(_task: @ffi.WaitableTask, x : UInt, y : UInt) -> Unit raise {
    assert_eq(x,5);
    assert_eq(y,6);
}

pub async fn async_two_arguments_and_result(_task: @ffi.WaitableTask, x : UInt, y : UInt) -> UInt raise {
    assert_eq(x,7);
    assert_eq(y,8);
    9
}

