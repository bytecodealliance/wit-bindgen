#ifndef _smw_wasm_h
#define _smw_wasm_h

/**
 * An attribute for making a function exported from the final Wasm binary.
 *
 * Example usage:
 *
 *     WASM_EXPORT
 *     int add(int a, int b) {
 *         return a + b;
 *     }
 */
#define WASM_EXPORT                             \
    __attribute__((visibility("default")))      \
    extern "C"

#endif // _smw_wasm_h
