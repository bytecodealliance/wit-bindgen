#include <middle_cpp.h>

using namespace test::common::test_types;

R1 exports::test::common::to_test::Wrap(F1 flag) {
    return ::test::common::to_test::Wrap(flag);
}

V1 exports::test::common::to_test::VarF(void) {
    return ::test::common::to_test::VarF();
}