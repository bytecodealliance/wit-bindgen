//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
  auto r0 = co_await test::async_ns::callbacks::InvokeAsync({0});
  assert(!r0.has_value());

  auto r1 = co_await test::async_ns::callbacks::InvokeAsync({1});
  assert(r1.has_value());
  assert(!r1->has_value());
  assert(r1->error().get_view() == "bad-id");

  auto r2 = co_await test::async_ns::callbacks::InvokeAsync({42});
  assert(r2.has_value());
  assert(r2->has_value());
  auto const& bytes = r2->value();
  assert(bytes.size() == 3);
  assert(bytes[0] == 42);
  assert(bytes[1] == 0xAB);
  assert(bytes[2] == 0xCD);
}
