(component
  (type (;0;) (func))
  (type (;1;) (list string))
  (type (;2;) (func (param "x" (type 1))))
  (type (;3;) (record (field "s" string)))
  (type (;4;) (func (param "x" (type 3))))
  (type (;5;) (variant (case "s" string)))
  (type (;6;) (func (param "x" (type 5))))
  (type (;7;) (record (field "s" u32)))
  (type (;8;) (func (param "x" (type 7))))
  (type (;9;) (variant (case "s" u32)))
  (type (;10;) (func (param "x" (type 9))))
  (type (;11;) (list (type 3)))
  (type (;12;) (func (param "x" (type 11))))
  (type (;13;) (list (type 5)))
  (type (;14;) (func (param "x" (type 13))))
  (type (;15;) (list u32))
  (type (;16;) (func (param "x" (type 15))))
  (type (;17;) (func (param "x" u32)))
  (type (;18;) (tuple u32 u32))
  (type (;19;) (func (result (type 18))))
  (type (;20;) (func (result string)))
  (type (;21;) (func (result (type 15))))
  (type (;22;) (func (result u32)))
  (type (;23;) (func (result (type 5))))
  (type (;24;) (list (type 9)))
  (type (;25;) (func (result (type 24))))
  (type (;26;) 
    (instance
      (alias outer 1 (type (;0;) 0))
      (export "a" (type 0))
      (alias outer 1 (type (;1;) 2))
      (export "b" (type 1))
      (alias outer 1 (type (;2;) 3))
      (export "r" (type 2))
      (alias outer 1 (type (;3;) 4))
      (export "c" (type 3))
      (alias outer 1 (type (;4;) 5))
      (export "v" (type 4))
      (alias outer 1 (type (;5;) 6))
      (export "d" (type 5))
      (alias outer 1 (type (;6;) 7))
      (export "r-no-string" (type 6))
      (alias outer 1 (type (;7;) 8))
      (export "e" (type 7))
      (alias outer 1 (type (;8;) 9))
      (export "v-no-string" (type 8))
      (alias outer 1 (type (;9;) 10))
      (export "f" (type 9))
      (alias outer 1 (type (;10;) 12))
      (export "g" (type 10))
      (alias outer 1 (type (;11;) 14))
      (export "h" (type 11))
      (alias outer 1 (type (;12;) 16))
      (export "i" (type 12))
      (alias outer 1 (type (;13;) 17))
      (export "j" (type 13))
      (alias outer 1 (type (;14;) 19))
      (export "k" (type 14))
      (alias outer 1 (type (;15;) 20))
      (export "l" (type 15))
      (alias outer 1 (type (;16;) 21))
      (export "m" (type 16))
      (alias outer 1 (type (;17;) 22))
      (export "n" (type 17))
      (alias outer 1 (type (;18;) 23))
      (export "o" (type 18))
      (alias outer 1 (type (;19;) 25))
      (export "p" (type 19))
    )
  )
  (module (;0;)
    (type (;0;) (func))
    (type (;1;) (func (param i32 i32)))
    (type (;2;) (func (param i32 i32 i32)))
    (type (;3;) (func (param i32)))
    (type (;4;) (func (result i32)))
    (type (;5;) (func (param i32 i32 i32 i32) (result i32)))
    (import "foo" "a" (func (;0;) (type 0)))
    (import "foo" "b" (func (;1;) (type 1)))
    (import "foo" "c" (func (;2;) (type 1)))
    (import "foo" "d" (func (;3;) (type 2)))
    (import "foo" "e" (func (;4;) (type 3)))
    (import "foo" "f" (func (;5;) (type 1)))
    (import "foo" "g" (func (;6;) (type 1)))
    (import "foo" "h" (func (;7;) (type 1)))
    (import "foo" "i" (func (;8;) (type 1)))
    (import "foo" "j" (func (;9;) (type 3)))
    (import "foo" "k" (func (;10;) (type 3)))
    (import "foo" "l" (func (;11;) (type 3)))
    (import "foo" "m" (func (;12;) (type 3)))
    (import "foo" "n" (func (;13;) (type 4)))
    (import "foo" "o" (func (;14;) (type 3)))
    (import "foo" "p" (func (;15;) (type 3)))
    (func (;16;) (type 5) (param i32 i32 i32 i32) (result i32)
      unreachable
    )
    (func (;17;) (type 2) (param i32 i32 i32)
      unreachable
    )
    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "canonical_abi_realloc" (func 16))
    (export "canonical_abi_free" (func 17))
  )
  (import "foo" (instance (;0;) (type 26)))
  (module (;1;)
    (type (;0;) (func))
    (type (;1;) (func (param i32 i32)))
    (type (;2;) (func (param i32 i32 i32)))
    (type (;3;) (func (param i32)))
    (type (;4;) (func (param i32)))
    (type (;5;) (func (result i32)))
    (type (;6;) (func (param i32)))
    (func (;0;) (type 0)
      i32.const 0
      call_indirect (type 0)
    )
    (func (;1;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 1
      call_indirect (type 1)
    )
    (func (;2;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 2
      call_indirect (type 1)
    )
    (func (;3;) (type 2) (param i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      i32.const 3
      call_indirect (type 2)
    )
    (func (;4;) (type 3) (param i32)
      local.get 0
      i32.const 4
      call_indirect (type 3)
    )
    (func (;5;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 5
      call_indirect (type 1)
    )
    (func (;6;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 6
      call_indirect (type 1)
    )
    (func (;7;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 7
      call_indirect (type 1)
    )
    (func (;8;) (type 1) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 8
      call_indirect (type 1)
    )
    (func (;9;) (type 3) (param i32)
      local.get 0
      i32.const 9
      call_indirect (type 3)
    )
    (func (;10;) (type 4) (param i32)
      local.get 0
      i32.const 10
      call_indirect (type 4)
    )
    (func (;11;) (type 4) (param i32)
      local.get 0
      i32.const 11
      call_indirect (type 4)
    )
    (func (;12;) (type 4) (param i32)
      local.get 0
      i32.const 12
      call_indirect (type 4)
    )
    (func (;13;) (type 5) (result i32)
      i32.const 13
      call_indirect (type 5)
    )
    (func (;14;) (type 6) (param i32)
      local.get 0
      i32.const 14
      call_indirect (type 6)
    )
    (func (;15;) (type 4) (param i32)
      local.get 0
      i32.const 15
      call_indirect (type 4)
    )
    (table (;0;) 16 16 funcref)
    (export "a" (func 0))
    (export "b" (func 1))
    (export "c" (func 2))
    (export "d" (func 3))
    (export "e" (func 4))
    (export "f" (func 5))
    (export "g" (func 6))
    (export "h" (func 7))
    (export "i" (func 8))
    (export "j" (func 9))
    (export "k" (func 10))
    (export "l" (func 11))
    (export "m" (func 12))
    (export "n" (func 13))
    (export "o" (func 14))
    (export "p" (func 15))
    (export "$imports" (table 0))
  )
  (module (;2;)
    (type (;0;) (func))
    (type (;1;) (func (param i32 i32)))
    (type (;2;) (func (param i32 i32 i32)))
    (type (;3;) (func (param i32)))
    (type (;4;) (func (param i32)))
    (type (;5;) (func (result i32)))
    (type (;6;) (func (param i32)))
    (import "" "a" (func (;0;) (type 0)))
    (import "" "b" (func (;1;) (type 1)))
    (import "" "c" (func (;2;) (type 1)))
    (import "" "d" (func (;3;) (type 2)))
    (import "" "e" (func (;4;) (type 3)))
    (import "" "f" (func (;5;) (type 1)))
    (import "" "g" (func (;6;) (type 1)))
    (import "" "h" (func (;7;) (type 1)))
    (import "" "i" (func (;8;) (type 1)))
    (import "" "j" (func (;9;) (type 3)))
    (import "" "k" (func (;10;) (type 4)))
    (import "" "l" (func (;11;) (type 4)))
    (import "" "m" (func (;12;) (type 4)))
    (import "" "n" (func (;13;) (type 5)))
    (import "" "o" (func (;14;) (type 6)))
    (import "" "p" (func (;15;) (type 4)))
    (import "" "$imports" (table (;0;) 16 16 funcref))
    (elem (;0;) (i32.const 0) func 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15)
  )
  (instance (;1;) (instantiate (module 1)))
  (instance (;2;) (instantiate (module 0) (with "foo" (instance 1))))
  (alias export (instance 1) "$imports" (table (;0;)))
  (alias export (instance 0) "a" (func (;0;)))
  (alias export (instance 0) "b" (func (;1;)))
  (alias export (instance 0) "c" (func (;2;)))
  (alias export (instance 0) "d" (func (;3;)))
  (alias export (instance 0) "e" (func (;4;)))
  (alias export (instance 0) "f" (func (;5;)))
  (alias export (instance 0) "g" (func (;6;)))
  (alias export (instance 0) "h" (func (;7;)))
  (alias export (instance 0) "i" (func (;8;)))
  (alias export (instance 0) "j" (func (;9;)))
  (alias export (instance 0) "k" (func (;10;)))
  (alias export (instance 0) "l" (func (;11;)))
  (alias export (instance 0) "m" (func (;12;)))
  (alias export (instance 0) "n" (func (;13;)))
  (alias export (instance 0) "o" (func (;14;)))
  (alias export (instance 0) "p" (func (;15;)))
  (func (;16;) (canon.lower (func 0)))
  (func (;17;) (canon.lower utf8 (into (instance 2)) (func 1)))
  (func (;18;) (canon.lower utf8 (into (instance 2)) (func 2)))
  (func (;19;) (canon.lower utf8 (into (instance 2)) (func 3)))
  (func (;20;) (canon.lower (func 4)))
  (func (;21;) (canon.lower (func 5)))
  (func (;22;) (canon.lower utf8 (into (instance 2)) (func 6)))
  (func (;23;) (canon.lower utf8 (into (instance 2)) (func 7)))
  (func (;24;) (canon.lower (into (instance 2)) (func 8)))
  (func (;25;) (canon.lower (func 9)))
  (func (;26;) (canon.lower (into (instance 2)) (func 10)))
  (func (;27;) (canon.lower utf8 (into (instance 2)) (func 11)))
  (func (;28;) (canon.lower (into (instance 2)) (func 12)))
  (func (;29;) (canon.lower (func 13)))
  (func (;30;) (canon.lower utf8 (into (instance 2)) (func 14)))
  (func (;31;) (canon.lower (into (instance 2)) (func 15)))
  (instance (;3;) core (export "$imports" (table 0)) (export "a" (func 16)) (export "b" (func 17)) (export "c" (func 18)) (export "d" (func 19)) (export "e" (func 20)) (export "f" (func 21)) (export "g" (func 22)) (export "h" (func 23)) (export "i" (func 24)) (export "j" (func 25)) (export "k" (func 26)) (export "l" (func 27)) (export "m" (func 28)) (export "n" (func 29)) (export "o" (func 30)) (export "p" (func 31)))
  (instance (;4;) (instantiate (module 2) (with "" (instance 3))))
)