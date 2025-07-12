#include <leaf_cpp.h>

using namespace test::common::test_types;

R1 exports::test::common::to_test::Wrap(F1 flag) {
    if (flag == F1::kA) {
        return R1{ 1, flag };
    } else {
        return R1{ 2, flag };
    }
}

V1 exports::test::common::to_test::VarF(void) {
    return V1(V1::B(42));
}
