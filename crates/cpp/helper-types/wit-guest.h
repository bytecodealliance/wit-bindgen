#include <string_view>
#include <stdint.h>
#include <malloc.h>

namespace wit {
    class string {
        uint8_t const* data;
        size_t length;
        public:
        string(string const&) = delete;
        string(string&&b) : data(b.data), length(b.length) {
            b.data=nullptr;
        }
        string& operator=(string const&) = delete;
        string& operator=(string &&b) {
            if (data) {free(const_cast<uint8_t*>(data));}
            data=b.data;
            length=b.length;
            b.data=nullptr;
            return *this;
        }
        ~string() {
            if (data) {
                free(const_cast<uint8_t*>(data));
            }
        }
        std::string_view get_view() const {
            return std::string_view((const char*)data, length);
        }
    };
}
