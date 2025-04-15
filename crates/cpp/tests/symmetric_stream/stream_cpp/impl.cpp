#include "stream_world_cpp.h"
#include "async_support.h"
#include <thread>
#include <stdlib.h>
#include <tuple>

wit::stream<uint32_t> exports::test::test::stream_test::Create() {
    stream_writer<uint32_t> wr{symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase(wit::ResourceImportBase::invalid))};
    wit::stream<uint32_t> rd{symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase(wit::ResourceImportBase::invalid))};
    std::tie(wr, rd) = create_wasi_stream<uint32_t>();
    // std::pair<stream_writer<uint32_t>, wit::stream<uint32_t>> my_stream = create_wasi_stream<uint32_t>();
    wit::stream<uint32_t> input = ::test::test::stream_source::Create();
    std::move(input).buffering(2).set_reader([stream = std::move(wr)](wit::span<uint32_t> data){
        for (auto i: data) {
            // stream.write(i);
            // stream.write(i+1);
        }
    });
    // while read, output value, value+1
    return rd;
}
