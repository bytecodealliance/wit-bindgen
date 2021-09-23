function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
}

export function f1(s) {
  assertEq(s, "Hello, WITX!");
}

export function f2() {
  return "36 chambers";
}

export function f3(a, b, c) {
  assertEq(a, "");
  assertEq(b, "🚀");
  assertEq(c, "hello");
  return [a, b, c];
}
