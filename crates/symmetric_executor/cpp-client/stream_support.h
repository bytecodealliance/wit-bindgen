#pragma once
#ifndef _STREAM_SUPPORT_H
#define _STREAM_SUPPORT_H

#include "module_cpp.h"
#include <functional>

namespace wit { 
    template <class T>
    union MaybeUninit
    {
        T value;
        char dummy;
        MaybeUninit()
            : dummy()
        { }
        MaybeUninit(MaybeUninit const& b)
            : dummy()
        { }
        ~MaybeUninit()
        { }
    };

    template<class T> struct stream {
        symmetric::runtime::symmetric_stream::StreamObj handle;

        uint32_t buffer_size = 1;

        static stream<T> new_empty() {
            return stream<T>{symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase()), 1};
        }
       
        struct background_object {
            symmetric::runtime::symmetric_stream::StreamObj handle;
            std::function<void(wit::span<T>)> reader;
            std::vector<MaybeUninit<T>> buffer;

            background_object(symmetric::runtime::symmetric_stream::StreamObj && h,
                std::function<void(wit::span<T>)>&& r, std::vector<MaybeUninit<T>> b) 
                : handle(std::move(h)), reader(std::move(r)), buffer(std::move(b)) {}
        };

        stream<T>& buffering(uint32_t amount) {
            buffer_size = amount;
            return *this;
        }
        static symmetric::runtime::symmetric_executor::CallbackState data_available(background_object* data) {
            auto buffer = data->handle.ReadResult();
            if (buffer.has_value()) {
                assert(buffer->GetAddress().into_handle() == (wit::ResourceImportBase::handle_t)data->buffer.data());
                uint32_t size = buffer->GetSize();
                if (size>0)
                    data->reader(wit::span<T>(&data->buffer[0].value, size));
                data->handle.StartReading(std::move(*buffer));
                return symmetric::runtime::symmetric_executor::CallbackState::kPending;
            } else {
                data->reader(wit::span<T>(&data->buffer[0].value, 0));
                auto release = std::unique_ptr<background_object>(data);
                return symmetric::runtime::symmetric_executor::CallbackState::kReady;
            }
        }
        void set_reader(std::function<void (wit::span<T>)> &&fun) && {
            std::vector<MaybeUninit<T>> buffer(buffer_size, MaybeUninit<T>());
            background_object* object = 
                std::make_unique<background_object>(background_object{std::move(handle), std::move(fun), std::move(buffer)}).release();

            symmetric::runtime::symmetric_stream::Buffer b(
                symmetric::runtime::symmetric_stream::Address(wit::ResourceImportBase{(wit::ResourceImportBase::handle_t)object->buffer.data()}), buffer_size);

            object->handle.StartReading(std::move(b));
            symmetric::runtime::symmetric_executor::Register(object->handle.ReadReadySubscribe(),
                symmetric::runtime::symmetric_executor::CallbackFunction(wit::ResourceImportBase{(wit::ResourceImportBase::handle_t)&data_available}),
                symmetric::runtime::symmetric_executor::CallbackData(wit::ResourceImportBase{(wit::ResourceImportBase::handle_t)object}));
        }
        stream(const stream&) = delete;
        stream(stream&&) = default;
        stream& operator=(const stream&) = delete;
        stream& operator=(stream&&) = default;
    }; 
}

#endif
