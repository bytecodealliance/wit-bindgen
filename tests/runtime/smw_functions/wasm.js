import * as imports from "imports";

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
}

export function test_imports() {
  // const { f1, f2, f3, f4, f5, f6 } = imports;
  const { f1, f2, f3, f4 } = imports;

  //
  // Testing arguments.
  //

  f1();

  f2(42);

  // Min and max `u32`.
  f3(0, 4294967295);

  //
  // Testing returns.
  //

  {
    const a = f4();
    assertEq(a, 1337);
  }

  // {
  //   const [a, b] = f5();
  //   assertEq(a, 1);
  //   assertEq(b, 2);
  // }

  // {
  //   const [a, b, c] = f6(100, 200, 300);
  //   assertEq(a, 101);
  //   assertEq(b, 201);
  //   assertEq(c, 301);
  // }
}

//
// Testing arguments.
//

export function f1() {}

export function f2(x) {
  assertEq(x, 42);
}

export function f3(a, b) {
  assertEq(a, 0);
  assertEq(b, 4294967295);
}

//
// Testing returns.
//

export function f4() {
  return 1337;
}

export function f5() {
  return [1, 2];
}

export function f6(a, b, c) {
  assertEq(a, 100);
  assertEq(b, 200);
  assertEq(c, 300);
  return [a + 1, b + 1, c + 1];
}
