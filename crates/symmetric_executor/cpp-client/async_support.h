
#include <future>
#include "module_cpp.h"

static symmetric::runtime::symmetric_executor::CallbackState fulfil_promise_void(void* data) {
    std::unique_ptr<std::promise<void>> ptr((std::promise<void>*)data);
    ptr->set_value();
    return symmetric::runtime::symmetric_executor::CallbackState::kReady;
}
  
std::future<void> lift_event(void* event) {
    std::promise<void> result;
    std::future<void> result1 = result.get_future();
    if (!event) { 
        result.set_value(); 
    } else {
        std::unique_ptr<std::promise<void>> ptr = std::make_unique<std::promise<void>>(std::move(result));
        symmetric::runtime::symmetric_executor::EventSubscription ev = symmetric::runtime::symmetric_executor::EventSubscription(wit::ResourceImportBase((uint8_t*)event));
        symmetric::runtime::symmetric_executor::CallbackFunction fun = symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)fulfil_promise_void));
        symmetric::runtime::symmetric_executor::CallbackData data = symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)ptr.release()));
        symmetric::runtime::symmetric_executor::Register(std::move(ev), std::move(fun), std::move(data));
    }
    return result1;
}

static symmetric::runtime::symmetric_executor::CallbackState wait_on_future(std::future<void>* fut) {
    fut->get();
    delete fut;
    return symmetric::runtime::symmetric_executor::CallbackState::kReady;
}  

template <class T>
void* lower_async(std::future<T> &&result1, std::function<void(T&&)> &&lower_result) {
    if (result1.wait_for(std::chrono::seconds::zero()) == std::future_status::ready) {
        lower_result(result1.get());
        return nullptr;
    } else {
        symmetric::runtime::symmetric_executor::EventGenerator gen;
        auto waiting = gen.Subscribe();
        auto task = std::async(std::launch::async, [lower_result](std::future<wit::string>&& result1, 
                symmetric::runtime::symmetric_executor::EventGenerator &&gen){
            lower_result(result1.get());
            gen.Activate();
        }, std::move(result1), std::move(gen));
        auto fut = std::make_unique<std::future<void>>(std::move(task));
        symmetric::runtime::symmetric_executor::Register(waiting.Dup(), 
            symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)wait_on_future)),
            symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)fut.release())));
        return waiting.into_handle();
    }    
}

template <class T>
struct fulfil_promise_data {
    symmetric::runtime::symmetric_stream::StreamObj stream;
    std::promise<T> promise;
};

template <class T>
static symmetric::runtime::symmetric_executor::CallbackState fulfil_promise() {
        // auto future = std::async(std::launch::async, [](std::future<T>&& fut, future_writer<T> &&wr){
    // }, std::move(f), std::move(handles.0));
    abort();
}

template <class T>
std::future<T> lift_future(uint8_t* stream) {
    std::promise<T> promise;
    std::future<T> result= promise.get_future();
    auto stream2 = symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase(stream));
    auto event = stream2.ReadReadySubscribe();
    std::unique_ptr<fulfil_promise_data<T>> data = std::make_unique<fulfil_promise_data<T>>(fulfil_promise_data<T>{std::move(stream2), std::move(promise)});
    symmetric::runtime::symmetric_executor::Register(std::move(event),
            symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)&fulfil_promise<T>)),
            symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)data.release())));
    return result;
}

template <class T> struct future_writer {
    symmetric::runtime::symmetric_stream::StreamObj handle;
};

template <class T> struct future_reader {
    symmetric::runtime::symmetric_stream::StreamObj handle;
};

template <class T>
std::pair<future_writer<T>, future_reader<T>> create_wasi_future() {
    auto stream = symmetric::runtime::symmetric_stream::StreamObj();
    auto stream2 = stream.Clone();
    return std::make_pair<future_writer<T>, future_reader<T>>(
        future_writer<T>{std::move(stream)}, future_reader<T>{std::move(stream2)});
}

template <class T>
struct write_to_future_data {
    future_writer<T> wr;
    std::future<T> fut;
};

template <class T>
static symmetric::runtime::symmetric_executor::CallbackState write_to_future(void* data) {
    std::unique_ptr<write_to_future_data<T>> ptr((write_to_future_data<T>*)data);
    auto result = ptr->fut.get();
    auto buffer = ptr->wr.handle.StartWriting();
    assert(buffer.GetSize()==sizeof(T));
    T* dataptr = (T*)(buffer.GetAddress().into_handle());
    new (dataptr) T(std::move(result));
    ptr->wr.handle.FinishWriting(std::optional<symmetric::runtime::symmetric_stream::Buffer>(std::move(buffer)));
}

template <class T>
uint8_t* lower_future(std::future<T> &&f) {
    auto handles = create_wasi_future<T>();
    auto wait_on = handles.first.handle.WriteReadySubscribe();
    auto fut = std::make_unique<write_to_future_data<T>>(write_to_future_data<T>{std::move(handles.first), std::move(f)});
    symmetric::runtime::symmetric_executor::Register(std::move(wait_on), 
        symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)&write_to_future<T>)),
        symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)fut.release())));
    return handles.second.handle.into_handle();
}
