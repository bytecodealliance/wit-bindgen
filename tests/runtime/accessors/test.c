#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <test.h>

static uint32_t COUNTER = 0;
static uint32_t BOUNDED = 0;
static uint64_t MAX_SIZE = 1024;

uint32_t exports_test_accessors_i_get_counter(void) {
    return COUNTER;
}

void exports_test_accessors_i_set_counter(uint32_t value) {
    COUNTER = value;
}

void exports_test_accessors_i_get_read_only(test_string_t *ret) {
    test_string_dup(ret, "read only");
}

uint32_t exports_test_accessors_i_get_bounded(void) {
    return BOUNDED;
}

bool exports_test_accessors_i_set_bounded(uint32_t value, test_string_t *err) {
    if (value > 100) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%" PRIu32 " is out of bounds", value);
        test_string_dup(err, buf);
        return false;
    }
    BOUNDED = value;
    return true;
}

struct exports_test_accessors_i_blob_t {
    test_list_u8_t contents;
    uint64_t position;
    test_string_t label;
};

exports_test_accessors_i_own_blob_t exports_test_accessors_i_constructor_blob(test_list_u8_t *init) {
    exports_test_accessors_i_blob_t *blob = (exports_test_accessors_i_blob_t *) malloc(sizeof(exports_test_accessors_i_blob_t));
    blob->contents = *init;
    blob->position = 0;
    test_string_dup(&blob->label, "");
    return exports_test_accessors_i_blob_new(blob);
}

void exports_test_accessors_i_method_blob_contents(exports_test_accessors_i_borrow_blob_t self, test_list_u8_t *ret) {
    ret->len = self->contents.len;
    ret->ptr = (uint8_t *) malloc(ret->len);
    memcpy(ret->ptr, self->contents.ptr, ret->len);
}

uint64_t exports_test_accessors_i_method_get_blob_position(exports_test_accessors_i_borrow_blob_t self) {
    return self->position;
}

void exports_test_accessors_i_method_set_blob_position(exports_test_accessors_i_borrow_blob_t self, uint64_t value) {
    self->position = value;
}

void exports_test_accessors_i_method_get_blob_label(exports_test_accessors_i_borrow_blob_t self, test_string_t *ret) {
    test_string_dup_n(ret, (const char *) self->label.ptr, self->label.len);
}

bool exports_test_accessors_i_method_set_blob_label(exports_test_accessors_i_borrow_blob_t self, test_string_t *value, test_string_t *err) {
    if (value->len == 0) {
        test_string_free(value);
        test_string_dup(err, "label must not be empty");
        return false;
    }
    test_string_free(&self->label);
    self->label = *value;
    return true;
}

uint64_t exports_test_accessors_i_static_get_blob_max_size(void) {
    return MAX_SIZE;
}

bool exports_test_accessors_i_static_set_blob_max_size(uint64_t value, test_string_t *err) {
    if (value == 0) {
        test_string_dup(err, "max size must be nonzero");
        return false;
    }
    MAX_SIZE = value;
    return true;
}

void exports_test_accessors_i_blob_destructor(exports_test_accessors_i_blob_t *rep) {
    test_list_u8_free(&rep->contents);
    test_string_free(&rep->label);
    free(rep);
}
