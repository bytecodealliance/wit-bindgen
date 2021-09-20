function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
}

export function f1(l) {
  assertEq(l.length, 3);
  assertEq(l[0], 1);
  assertEq(l[1], 2);
  assertEq(l[2], 3);
}

export function f2() {
  return [1, 2, 3];
}

export function f3(a, b) {
  assertEq(a.length, 0);
  assertEq(b.length, 3);
  assertEq(b[0], 1);
  assertEq(b[1], 2);
  assertEq(b[2], 3);
  return [
    [],
    [1, 2, 3]
  ];
}

export function f4(l) {
  assertEq(l.length, 3);
  assertEq(l[0].length, 0);
  assertEq(l[1].length, 1);
  assertEq(l[1][0], 1);
  assertEq(l[2].length, 2);
  assertEq(l[2][0], 2);
  assertEq(l[2][1], 3);
  return [
    [],
    [4],
    [5, 6]
  ];
}
