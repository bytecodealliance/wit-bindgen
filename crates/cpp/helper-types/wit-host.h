#pragma once

#include <string_view>
#include <stdint.h>
#include <malloc.h>

#ifndef WIT_HOST_DIRECT
#define WIT_HOST_WAMR
#endif

#ifdef WIT_HOST_DIRECT
# define WIT_WASI64
#endif

namespace wit {
#ifdef WIT_WASI64
    typedef uint64_t guest_address;
    typedef uint64_t guest_size;
#else
    typedef uint32_t guest_address;
    typedef uint32_t guest_size;
#endif

#ifdef WIT_HOST_WAMR
    typedef void* guest_instance;
#elif defined(WIT_HOST_DIRECT)
    typedef int guest_instance;
#endif
    typedef void (*guest_cabi_post_t)(guest_instance, guest_address);
    typedef void* (*from_guest_address_t)(guest_instance, guest_address);
    
    // host code never de-allocates directly
    class string {
        guest_address data_;
        guest_size length;
        public:
        // string(string const&) = default;
        // string(string&&b) = default;
        // string& operator=(string const&) = default;
        // string& operator=(string &&b) = default;
        // ~string() {}
        std::string_view get_view(from_guest_address_t conv, guest_instance inst) const {
            return std::string_view((char const*)(*conv)(inst, data_), length);
        }
        string(guest_address a, guest_size s) : data_(a), length(s) {}
    };

    template <class T>
    class guest_owned {
        guest_address data_;
        guest_cabi_post_t free_func;
        from_guest_address_t conv_func;
        guest_instance instance;
        public:
        T const* operator->() const {
            return (T const*)(*conv_func)(instance, data_);
        }
        T* operator->() {
            return (T*)(*conv_func)(instance, data_);
        }
    };
}
