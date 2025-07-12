#include <assert.h>
#include <runner_cpp.h>

int main() {
    using namespace ::test::common::test_types;
    
    R1 res = test::common::to_test::Wrap(F1::kA);
    assert(res.b == F1::kA);
    assert(res.a == 1);

    R1 res2 = test::common::to_test::Wrap(F1::kB);
    assert(res2.b == F1::kB);
    assert(res2.a == 2);

    V1 res3 = test::common::to_test::VarF();
    assert(res3.variants.index() == 1);
    assert(std::get<1>(res3.variants).value == 42);

    return 0;
}
