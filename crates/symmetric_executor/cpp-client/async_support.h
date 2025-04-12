
#include <future>
#include "module_cpp.h"

static symmetric::runtime::symmetric_executor::CallbackState fulfil_promise(void* data) {
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
        symmetric::runtime::symmetric_executor::CallbackFunction fun = symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase((uint8_t*)fulfil_promise));
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
std::future<T> lift_future(uint8_t* stream) {

}

template <class T>
uint8_t* lower_future(std::future<T> &&f) {

}
