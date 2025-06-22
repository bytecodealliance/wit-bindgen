#include <async_support.h>
#include <test_cpp.h>
#include <thread>

static constexpr uint32_t SIZE = 5;
static const char *(pattern)[SIZE] = { 
    "Hello", "World!", "From", "a", "stream."
};
static uint32_t next = 0;

static symmetric::runtime::symmetric_executor::CallbackState ready_to_write(stream_writer<wit::string>* data) {
    if (next<SIZE) {
        data->write(std::vector<wit::string>(1, wit::string::from_view(pattern[next])));
        ++next;
        return symmetric::runtime::symmetric_executor::CallbackState::kPending;
    } else {
        data->write(std::vector<wit::string>());
        auto release = std::unique_ptr<stream_writer<wit::string>>(data);
        return symmetric::runtime::symmetric_executor::CallbackState::kReady;
    }
}

wit::stream<wit::string> exports::a::b::the_test::F() {
    auto streampair = create_wasi_stream<wit::string>();
#if 0
    stream_writer<wit::string>* streampointer = std::make_unique<stream_writer<wit::string>>(std::move(std::move(streampair).first)).release();

    std::async(std::launch::async, [streampointer](){
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        streampointer->write(std::vector<wit::string>(1, wit::string::from_view("Hello")));
        auto release = std::unique_ptr<stream_writer<wit::string>>(streampointer);
    });
    // TODO: How to handle the returned future
#elif 1
    stream_writer<wit::string>* streampointer = std::make_unique<stream_writer<wit::string>>(std::move(std::move(streampair).first)).release();
    // manual handling for now
    symmetric::runtime::symmetric_executor::Register(streampointer->handle.WriteReadySubscribe(), 
            symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase{(wit::ResourceImportBase::handle_t)&ready_to_write}),
            symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase{(wit::ResourceImportBase::handle_t)streampointer})
    );
#else
    std::vector<wit::string> feed;
    feed.push_back(wit::string::from_view("Hello"));
    feed.push_back(wit::string::from_view("World!"));
    feed.push_back(wit::string::from_view("From"));
    feed.push_back(wit::string::from_view("a"));
    feed.push_back(wit::string::from_view("stream."));
    streampair.first.write(std::move(feed));
    // TODO: Blocking doesn't work well in this test
#endif
    return std::move(streampair).second;
}
