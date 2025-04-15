#pragma once
#ifndef _STREAM_SUPPORT_H
#define _STREAM_SUPPORT_H

#include "module_cpp.h"
#include <functional>

namespace wit { template<class T> struct stream {
    symmetric::runtime::symmetric_stream::StreamObj handle;

    stream<T> buffering(uint32_t amount) &&;
    void set_reader(std::function<void (wit::span<T>)>) &&;
}; }

#endif
