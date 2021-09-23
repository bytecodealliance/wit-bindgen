import { f1, f2, f3, f4, f5, f6 } from "functions";

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

{
  const [a, b] = f5();
  assertEq(a, 1);
  assertEq(b, 2);
}

{
  const [a, b, c] = f6(100, 200, 300);
  assertEq(a, 101);
  assertEq(b, 201);
  assertEq(c, 301);
}
