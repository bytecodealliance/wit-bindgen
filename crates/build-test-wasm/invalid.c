#include <stdint.h>

// Testing invalid arguments to imports

__attribute__((import_module("host"), import_name("invert_bool")))
uint32_t bad_bool(uint32_t val);

__attribute__((export_name("invalid_bool")))
void invalid_bool() {
  bad_bool(2);
}

__attribute__((import_module("host"), import_name("roundtrip_char")))
uint32_t bad_char(uint32_t val);

__attribute__((export_name("invalid_char")))
void invalid_char() {
  bad_char(0xd800);
}

__attribute__((import_module("host"), import_name("roundtrip_u8")))
uint32_t bad_u8(uint32_t val);

__attribute__((export_name("invalid_u8")))
void invalid_u8() {
  bad_u8(~0);
}

__attribute__((import_module("host"), import_name("roundtrip_s8")))
uint32_t bad_s8(uint32_t val);

__attribute__((export_name("invalid_s8")))
void invalid_s8() {
  bad_s8(1 << 30);
}

__attribute__((import_module("host"), import_name("roundtrip_u16")))
uint32_t bad_u16(uint32_t val);

__attribute__((export_name("invalid_u16")))
void invalid_u16() {
  bad_u16(~0);
}

__attribute__((import_module("host"), import_name("roundtrip_s16")))
uint32_t bad_s16(uint32_t val);

__attribute__((export_name("invalid_s16")))
void invalid_s16() {
  bad_s16(1 << 30);
}

__attribute__((import_module("host"), import_name("roundtrip_enum")))
uint32_t bad_e1(uint32_t val);

__attribute__((export_name("invalid_e1")))
void invalid_e1() {
  bad_e1(400);
}

__attribute__((import_module("host"), import_name("host_state_get")))
uint32_t bad_handle(uint32_t val);

__attribute__((export_name("invalid_handle")))
void invalid_handle() {
  bad_handle(100);
}

__attribute__((import_module("canonical_abi"), import_name("resource_drop_host_state2")))
void bad_close(uint32_t val);

__attribute__((export_name("invalid_handle_close")))
void invalid_handle_close() {
  bad_close(100);
}
