// Generated by `wit-bindgen` 0.3.0. DO NOT EDIT!
#ifndef __CPP_GUEST_BINDINGS_THE_WORLD_H
#define __CPP_GUEST_BINDINGS_THE_WORLD_H
#include <cassert>
#include <cstdint>
#include <map>
#include <utility>
#include <memory>
#include <wit-guest.h>
namespace foo {
namespace foo {
namespace resources {
class R : public wit::ResourceImportBase {

public:
  ~R();
  R(uint32_t a);
  void Add(uint32_t b) const;
  R(wit::ResourceImportBase &&);

  R(R &&) = default;
};

R Create();
void Borrows(std::reference_wrapper<const R> o);
void Consume(R &&o);
// export_interface Interface(Id { idx: 0 })
} // namespace resources
} // namespace foo
} // namespace foo
#include "exports-foo-foo-resources-R.h"
namespace exports {
namespace foo {
namespace foo {
namespace resources {
std::unique_ptr<R, R::Deleter> Create();
void Borrows(std::reference_wrapper<const R> o);
void Consume(std::unique_ptr<R, R::Deleter> o);
} // namespace resources
} // namespace foo
} // namespace foo
} // namespace exports

#endif