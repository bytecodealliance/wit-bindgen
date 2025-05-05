#pragma once
#ifndef _STREAM_SUPPORT_H
#define _STREAM_SUPPORT_H

#include "module_cpp.h"
#include <functional>

namespace wit { 
    // template <class T>
    // union MaybeUninit
    // {
    //     T value;
    //     char dummy;
    //     MaybeUninit()
    //         : dummy()
    //     { }
    //     MaybeUninit(MaybeUninit const& b)
    //         : dummy()
    //     { }
    //     ~MaybeUninit()
    //     { }
    // };

    template <class T>
    struct StreamProperties {
        static const uint32_t lowered_size;
        static T lift(uint8_t const*);
        static void lower(T&&, uint8_t*);
    };

    template<class T> struct stream {
        symmetric::runtime::symmetric_stream::StreamObj handle;
        // StreamProperties<T> const* lifting;

        uint32_t buffer_size = 1;

        static stream<T> new_empty() {
            return stream<T>{symmetric::runtime::symmetric_stream::StreamObj(wit::ResourceImportBase()), 1};
        }
       
        struct background_object {
            symmetric::runtime::symmetric_stream::StreamObj handle;
            std::function<void(wit::span<T>)> reader;
            std::vector<uint8_t> buffer;
            // StreamProperties<T> const* lifting;

            background_object(symmetric::runtime::symmetric_stream::StreamObj && h,
                std::function<void(wit::span<T>)>&& r, std::vector<uint8_t> &&b)
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
                std::vector<T> lifted;
                lifted.reserve(size);
                for (uint32_t i = 0; i<size; ++i) {
                    lifted.push_back(StreamProperties<T>::lift(data->buffer.data()+i*StreamProperties<T>::lowered_size));
                }
                if (size>0)
                    data->reader(wit::span<T>(lifted.data(), size));
                data->handle.StartReading(std::move(*buffer));
                return symmetric::runtime::symmetric_executor::CallbackState::kPending;
            } else {
                data->reader(wit::span<T>(nullptr, 0));
                auto release = std::unique_ptr<background_object>(data);
                return symmetric::runtime::symmetric_executor::CallbackState::kReady;
            }
        }
        void set_reader(std::function<void (wit::span<T>)> &&fun) && {
            std::vector<uint8_t> buffer(buffer_size*StreamProperties<T>::lowered_size, uint8_t(0));
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
