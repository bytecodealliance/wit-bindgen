//@ args = '--with my:inline/foo=alien.h'

#include <runner_cpp.h>

void exports::runner::Run() {
    auto msg = my::inline_::foo::Msg { wit::string::from_view("hello") };
    my::inline_::bar::Bar(msg);
}
