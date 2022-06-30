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
  const { f1, f2 } = imports;
  // const { f1, f2, f3 } = imports;
  f1("Hello, WIT!");

  const s = f2();
  assertEq(s, "36 chambers");

  // const [a, b, c] = f3("", "🚀", "hello");
  // assertEq(a, "");
  // assertEq(b, "🚀");
  // assertEq(c, "hello");
}

export function f1(s) {
  assertEq(s, "Hello, WIT!");
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
