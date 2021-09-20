function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
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
