#include <stream_support.h>
#include <test_cpp.h>
#include <thread>

wit::stream<wit::string> exports::a::b::the_test::F() {
    auto streampair = create_wasi_stream<wit::string>();
#if 1
    stream_writer<wit::string>* streampointer = std::make_unique<stream_writer<wit::string>>(std::move(std::move(streampair).first)).release();

    std::async(std::launch::async, [streampointer](){
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        auto release = std::unique_ptr<stream_writer<wit::string>>(streampointer);
    });
#else
    std::vector<wit::string> feed;
    feed.push_back(wit::string::from_view("Hello"));
    feed.push_back(wit::string::from_view("World!"));
    feed.push_back(wit::string::from_view("From"));
    feed.push_back(wit::string::from_view("a"));
    feed.push_back(wit::string::from_view("stream."));
    streampair.first.write(std::move(feed));
#endif
    return streampair.second;
}
