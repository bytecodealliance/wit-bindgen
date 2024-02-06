#include <string_view>
#include <stdint.h>
#include <malloc.h>

namespace wit {
    class string : public std::string_view {
        uint8_t * owned;
        public:
        string(string const&) = delete;
        string(string&&b) : std::string_view(b.data(), b.length()), owned(b.owned) {
            b.owned=nullptr;
        }
        string& operator=(string const&) = delete;
        string& operator=(string &&b) {
            if (owned) {free(owned);}
            *static_cast<std::string_view*>(this) = b;
            owned=b.owned;
            b.owned=nullptr;
        }
        ~string() {
            free(owned);
        }
    };
}
