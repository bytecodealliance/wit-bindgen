#include "stream_world_cpp.h"
#include "async_support.h"
#include <thread>
#include <stdlib.h>
#include <tuple>

wit::stream<uint32_t> exports::test::test::stream_test::Create() {
    auto streampair = create_wasi_stream<uint32_t>();
    stream_writer<uint32_t>* streampointer = std::make_unique<stream_writer<uint32_t>>(std::move(std::move(streampair).first)).release();
    wit::stream<uint32_t> input = ::test::test::stream_source::Create();
    input.buffering(2);
    std::move(input).set_reader([streampointer](wit::span<uint32_t> data){
        if (!data.empty()) {
            std::vector<uint32_t> feed;
            feed.reserve(data.size()*2);
            for (auto i: data) {
                feed.push_back(i);
                feed.push_back(i+1);
            }
            streampointer->write(std::move(feed));
        } else {
            // free the stream at EOF
            auto release = std::unique_ptr<stream_writer<uint32_t>>(streampointer);
        }
    });
    return std::move(streampair).second;
}
