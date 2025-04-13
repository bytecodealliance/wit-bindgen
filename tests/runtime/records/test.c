#include <assert.h>
#include "test.h"

void exports_test_records_to_test_multiple_results(test_tuple2_u8_u16_t *ret) {
    ret->f0 = 4;
    ret->f1 = 5;
  }
  
  void exports_test_records_to_test_swap_tuple(test_tuple2_u8_u32_t *a, test_tuple2_u32_u8_t *b) {
    b->f0 = a->f1;
    b->f1 = a->f0;
  }
  
  exports_test_records_to_test_f1_t exports_test_records_to_test_roundtrip_flags1(exports_test_records_to_test_f1_t a) {
    return a;
  }
  
  exports_test_records_to_test_f2_t exports_test_records_to_test_roundtrip_flags2(exports_test_records_to_test_f2_t a) {
    return a;
  }
  
  void exports_test_records_to_test_roundtrip_flags3(
        exports_test_records_to_test_flag8_t a,
        exports_test_records_to_test_flag16_t b,
        exports_test_records_to_test_flag32_t c,
        exports_test_records_to_test_tuple3_flag8_flag16_flag32_t *ret) {
    ret->f0 = a;
    ret->f1 = b;
    ret->f2 = c;
  }
  
  void exports_test_records_to_test_roundtrip_record1(exports_test_records_to_test_r1_t *a, exports_test_records_to_test_r1_t *ret0) {
    *ret0 = *a;
  }
  
  void exports_test_records_to_test_tuple1(test_tuple1_u8_t *a, test_tuple1_u8_t *b) {
    b->f0 = a->f0;
  }
  