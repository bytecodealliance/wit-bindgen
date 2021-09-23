import { f1, f2, f3 } from "strings";

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
}

f1("Hello, WITX!");

const s = f2();
assertEq(s, "36 chambers");

const [a, b, c] = f3("", "🚀", "hello");
assertEq(a, "");
assertEq(b, "🚀");
assertEq(c, "hello");
