
#include <future>
#include "module_cpp.h"
#include "stream_support.h"

// internal calback used by lift_event
static inline symmetric::runtime::symmetric_executor::CallbackState fulfil_promise_void(void* data) {
    std::unique_ptr<std::promise<void>> ptr((std::promise<void>*)data);
    ptr->set_value();
    return symmetric::runtime::symmetric_executor::CallbackState::kReady;
}
  
static inline std::future<void> lift_event(void* event) {
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

// internal callback for lower_async
static inline symmetric::runtime::symmetric_executor::CallbackState wait_on_future(std::future<void>* fut) {
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
        // move to run_in_background
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

// internal data structure used by lift_future
template <class T, class LIFT>
struct fulfil_promise_data {
    symmetric::runtime::symmetric_stream::StreamObj stream;
    std::promise<T> promise;
    uint8_t value[LIFT::SIZE];
};

// internal callback used by lift_future
template <class T, class LIFT>
static symmetric::runtime::symmetric_executor::CallbackState fulfil_promise(void* data) {
    std::unique_ptr<fulfil_promise_data<T, LIFT>> ptr((fulfil_promise_data<T, LIFT>*)data);
    auto buffer = ptr->stream.ReadResult();
    ptr->promise.set_value(LIFT::lift(ptr->value));
    return symmetric::runtime::symmetric_executor::CallbackState::kReady;
}

template <class T, class LIFT>
std::future<T> lift_future(uint8_t* stream) {
    std::promise<T> promise;
    std::future<T> result= promise.get_future();
    auto stream2 = symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase(stream));
    auto event = stream2.ReadReadySubscribe();
    std::unique_ptr<fulfil_promise_data<T, LIFT>> data = std::make_unique<fulfil_promise_data<T, LIFT>>(fulfil_promise_data<T, LIFT>{std::move(stream2), std::move(promise), {0}});
    symmetric::runtime::symmetric_stream::Buffer buf = symmetric::runtime::symmetric_stream::Buffer(
        symmetric::runtime::symmetric_stream::Address(wit::ResourceImportBase((wit::ResourceImportBase::handle_t)&data->value)),
        1
    );
    data->stream.StartReading(std::move(buf));
    symmetric::runtime::symmetric_executor::Register(std::move(event),
            symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)&fulfil_promise<T, LIFT>)),
            symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)data.release())));
    return result;
}

template <class T>
wit::stream<T> lift_stream(uint8_t* stream) {
    return wit::stream<T>{symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase((wit::ResourceImportBase::handle_t)stream))};
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

template <class T> struct stream_writer {
    symmetric::runtime::symmetric_stream::StreamObj handle;

    // non blocking write, returns remaining data
    std::vector<T> write_nb(std::vector<T> data) {
        auto buffer = handle.StartWriting();
        auto capacity = buffer.Capacity();
        uint8_t* dest = (uint8_t*)buffer.GetAddress().into_handle();
        auto elements = data.size();
        if (elements<capacity) elements=capacity;
        for (uint32_t i = 0; i<elements; ++i) {
            wit::StreamProperties<T>::lower(std::move(data[i]), dest+(i*wit::StreamProperties<T>::lowered_size));
        }
        buffer.SetSize(elements);
        handle.FinishWriting(std::optional<symmetric::runtime::symmetric_stream::Buffer>(std::move(buffer)));

        if (capacity>data.size()) capacity = data.size();
        data.erase(data.begin(), data.begin() + capacity);
        return data;
    }

    void write(std::vector<T>&& data) {
        while (!data.empty()) {
            if (!IsReadyToWrite()) {
                symmetric::runtime::symmetric_executor::BlockOn(handle.WriteReadySubscribe());
            }
            data = write_nb(std::move(data));
        }
    }
    bool IsReadyToWrite() const {
        return handle.IsReadyToWrite();
    }
    symmetric::runtime::symmetric_executor::EventSubscription WriteReadySubscribe() const {
        return handle.WriteReadySubscribe();
    }

    ~stream_writer() {
        if (handle.get_handle()!=wit::ResourceImportBase::invalid) {
            handle.FinishWriting(std::optional<symmetric::runtime::symmetric_stream::Buffer>());
        }
    }
    stream_writer(symmetric::runtime::symmetric_stream::StreamObj &&h) : handle(std::move(h)) {}
    stream_writer(const stream_writer&) = delete;
    stream_writer& operator=(const stream_writer&) = delete;
    stream_writer(stream_writer&&) = default;
    stream_writer& operator=(stream_writer&&) = default;
};

template <class T>
std::pair<stream_writer<T>, wit::stream<T>> create_wasi_stream() {
    auto stream = symmetric::runtime::symmetric_stream::StreamObj();
    auto stream2 = stream.Clone();
    return std::make_pair<stream_writer<T>, wit::stream<T>>(
        stream_writer<T>{std::move(stream)}, wit::stream<T>{std::move(stream2)});
}

// internal struct used by lower_future
template <class T>
struct write_to_future_data {
    future_writer<T> wr;
    std::future<T> fut;
};

// internal function used by lower_future
template <class T, class LOWER>
static symmetric::runtime::symmetric_executor::CallbackState write_to_future(void* data) {
    std::unique_ptr<write_to_future_data<T>> ptr((write_to_future_data<T>*)data);
    // is future ready?
    if (ptr->fut.wait_for(std::chrono::seconds::zero()) == std::future_status::ready) {
        auto buffer = ptr->wr.handle.StartWriting();
        assert(buffer.Capacity()==1);
        uint8_t* dataptr = (uint8_t*)(buffer.GetAddress().into_handle());
        auto result = ptr->fut.get();
        LOWER::lower(std::move(result), dataptr);
        buffer.SetSize(1);
        ptr->wr.handle.FinishWriting(std::optional<symmetric::runtime::symmetric_stream::Buffer>(std::move(buffer)));
    } else {
        // sadly there is no easier way to wait for a future in the background?
        // move to run_in_background
        symmetric::runtime::symmetric_executor::EventGenerator gen;
        auto waiting = gen.Subscribe();
        auto task = std::async(std::launch::async, [](std::unique_ptr<write_to_future_data<T>> &&ptr, 
            symmetric::runtime::symmetric_executor::EventGenerator &&gen){
            auto buffer = ptr->wr.handle.StartWriting();
            // assert(buffer.GetSize()==1); //sizeof(T));
            uint8_t* dataptr = (uint8_t*)(buffer.GetAddress().into_handle());        
            auto result = ptr->fut.get();
            LOWER::lower(std::move(result), dataptr);
            buffer.SetSize(1);
            ptr->wr.handle.FinishWriting(std::optional<symmetric::runtime::symmetric_stream::Buffer>(std::move(buffer)));
            gen.Activate();
        }, std::move(ptr), std::move(gen));
        auto fut = std::make_unique<std::future<void>>(std::move(task));
        symmetric::runtime::symmetric_executor::Register(waiting.Dup(), 
            symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)wait_on_future)),
            symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)fut.release())));
    }
    return symmetric::runtime::symmetric_executor::CallbackState::kReady;
}

template <class T, class LOWER>
uint8_t* lower_future(std::future<T> &&f) {
    auto handles = create_wasi_future<T>();
    auto wait_on = handles.first.handle.WriteReadySubscribe();
    auto fut = std::make_unique<write_to_future_data<T>>(write_to_future_data<T>{std::move(handles.first), std::move(f)});
    symmetric::runtime::symmetric_executor::Register(std::move(wait_on), 
        symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)&write_to_future<T, LOWER>)),
        symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase((uint8_t*)fut.release())));
    return handles.second.handle.into_handle();
}

template <class T>
uint8_t* lower_stream(wit::stream<T> &&f) {
    return f.handle.into_handle();
}
