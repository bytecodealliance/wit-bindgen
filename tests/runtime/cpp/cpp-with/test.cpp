#include <assert.h>
#include <test_cpp.h>

void exports::my::inline_::bar::Bar(::my::inline_::foo::Msg m) {
    assert(m.field.get_view() == "hello");
}
