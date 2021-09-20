import { f1, f2, f3, f4 } from "lists";

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEq(a, b) {
  assert(a == b, `assertEq failed: ${a} != ${b}`);
}

f1([1, 2, 3]);

const l = f2();
assertEq(l.length, 3);
assertEq(l[0], 1);
assertEq(l[1], 2);
assertEq(l[2], 3);

const [a, b] = f3([], [1, 2, 3]);
assertEq(a.length, 0);
assertEq(b.length, 3);
assertEq(b[0], 1);
assertEq(b[1], 2);
assertEq(b[2], 3);

const l2 = f4([[], [1], [2, 3]]);
assertEq(l2.length, 3);
assertEq(l2[0].length, 0);
assertEq(l2[1].length, 1);
assertEq(l2[1][0], 4);
assertEq(l2[2].length, 2);
assertEq(l2[2][0], 5);
assertEq(l2[2][1], 6);
