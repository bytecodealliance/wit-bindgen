//@ wasmtime-flags = '-Wcomponent-model-map'

#include <assert.h>
#include <runner_cpp.h>

#include <cstring>
#include <string>
#include <vector>

namespace to_test = ::test::maps::to_test;

using wit::string;
using wit::unordered_map;

static bool str_eq(string const& a, std::string_view b) { return a.get_view() == b; }

template <class V>
static V const* find(unordered_map<string, V> const& m, std::string_view key) {
    for (auto const& [k, v] : m) {
        if (k.get_view() == key) {
            return &v;
        }
    }
    return nullptr;
}

template <class V>
static V const* find(unordered_map<uint32_t, V> const& m, uint32_t key) {
    for (auto const& [k, v] : m) {
        if (k == key) {
            return &v;
        }
    }
    return nullptr;
}

static unordered_map<uint32_t, string> make_names(
    std::initializer_list<std::pair<uint32_t, std::string_view>> entries) {
    auto result = unordered_map<uint32_t, string>::allocate(entries.size());
    size_t i = 0;
    for (auto const& [key, value] : entries) {
        result.initialize(i++, std::make_pair(key, string::from_view(value)));
    }
    return result;
}

static void test_named_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> input{{1, "uno"}, {2, "two"}};
    auto result = to_test::NamedRoundtrip(input);
    assert(result.size() == 2);
    auto uno = find(result, "uno");
    assert(uno && *uno == 1);
    auto two = find(result, "two");
    assert(two && *two == 2);
}

static void test_bytes_roundtrip() {
    uint8_t const world_bytes[] = {'w', 'o', 'r', 'l', 'd'};
    uint8_t const bin_bytes[] = {0, 1, 2};
    std::vector<std::pair<std::string_view, std::span<uint8_t const>>> input{
        {"hello", world_bytes},
        {"bin", bin_bytes},
    };
    auto result = to_test::BytesRoundtrip(input);
    assert(result.size() == 2);
    auto hello = find(result, "hello");
    assert(hello && hello->size() == 5);
    assert(memcmp(hello->data(), "world", 5) == 0);
    auto bin = find(result, "bin");
    assert(bin && bin->size() == 3);
    assert((*bin)[0] == 0 && (*bin)[1] == 1 && (*bin)[2] == 2);
}

static void test_empty_roundtrip() {
    auto result = to_test::EmptyRoundtrip({});
    assert(result.empty());
}

static void test_option_roundtrip() {
    std::vector<std::pair<std::string_view, std::optional<uint32_t>>> input{
        {"some", 42},
        {"none", std::nullopt},
    };
    auto result = to_test::OptionRoundtrip(input);
    assert(result.size() == 2);
    auto some = find(result, "some");
    assert(some && some->has_value() && **some == 42);
    auto none = find(result, "none");
    assert(none && !none->has_value());
}

static void test_record_roundtrip() {
    to_test::LabeledEntry input{
        string::from_view("test-label"),
        make_names({{10, "ten"}, {20, "twenty"}}),
    };
    auto result = to_test::RecordRoundtrip(std::move(input));
    assert(str_eq(result.label, "test-label"));
    assert(result.values.size() == 2);
    auto ten = find(result.values, 10);
    assert(ten && str_eq(*ten, "ten"));
    auto twenty = find(result.values, 20);
    assert(twenty && str_eq(*twenty, "twenty"));
}

static void test_inline_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> input{{1, "one"}, {2, "two"}};
    auto result = to_test::InlineRoundtrip(input);
    assert(result.size() == 2);
    auto one = find(result, "one");
    assert(one && *one == 1);
    auto two = find(result, "two");
    assert(two && *two == 2);
}

static void test_large_roundtrip() {
    size_t const n = 100;
    std::vector<std::string> storage;
    storage.reserve(n);
    std::vector<std::pair<uint32_t, std::string_view>> input;
    input.reserve(n);
    for (size_t i = 0; i < n; i++) {
        storage.push_back("value-" + std::to_string(i));
        input.emplace_back(uint32_t(i), storage.back());
    }
    auto result = to_test::LargeRoundtrip(input);
    assert(result.size() == n);
    auto value = find(result, 42);
    assert(value && str_eq(*value, "value-42"));
}

static void test_multi_param_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> names{{1, "one"}, {2, "two"}};
    uint8_t const payload[] = {42};
    std::vector<std::pair<std::string_view, std::span<uint8_t const>>> bytes{{"key", payload}};
    auto [ids, bytes_out] = to_test::MultiParamRoundtrip(names, bytes);
    assert(ids.size() == 2);
    auto one = find(ids, "one");
    assert(one && *one == 1);
    auto two = find(ids, "two");
    assert(two && *two == 2);
    assert(bytes_out.size() == 1);
    auto key = find(bytes_out, "key");
    assert(key && key->size() == 1 && (*key)[0] == 42);
}

static void test_nested_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> inner_a{{1, "one"}, {2, "two"}};
    std::vector<std::pair<uint32_t, std::string_view>> inner_b{{10, "ten"}};
    std::vector<std::pair<std::string_view, std::span<std::pair<uint32_t, std::string_view> const>>>
        outer{
            {"group-a", inner_a},
            {"group-b", inner_b},
        };
    auto result = to_test::NestedRoundtrip(outer);
    assert(result.size() == 2);
    auto group_a = find(result, "group-a");
    assert(group_a && group_a->size() == 2);
    auto two = find(*group_a, 2);
    assert(two && str_eq(*two, "two"));
    auto group_b = find(result, "group-b");
    assert(group_b && group_b->size() == 1);
    auto ten = find(*group_b, 10);
    assert(ten && str_eq(*ten, "ten"));
}

static void test_variant_roundtrip() {
    to_test::MapOrString map_input{to_test::MapOrString::AsMap{make_names({{1, "one"}})}};
    auto map_result = to_test::VariantRoundtrip(std::move(map_input));
    auto* as_map = std::get_if<to_test::MapOrString::AsMap>(&map_result.variants);
    assert(as_map && as_map->value.size() == 1);
    auto one = find(as_map->value, 1);
    assert(one && str_eq(*one, "one"));

    to_test::MapOrString string_input{to_test::MapOrString::AsString{string::from_view("hello")}};
    auto string_result = to_test::VariantRoundtrip(std::move(string_input));
    auto* as_string = std::get_if<to_test::MapOrString::AsString>(&string_result.variants);
    assert(as_string && str_eq(as_string->value, "hello"));
}

static void test_result_roundtrip() {
    std::expected<unordered_map<uint32_t, string>, string> ok_input{make_names({{5, "five"}})};
    auto ok_result = to_test::ResultRoundtrip(std::move(ok_input));
    assert(ok_result.has_value());
    assert(ok_result->size() == 1);
    auto five = find(*ok_result, 5);
    assert(five && str_eq(*five, "five"));

    std::expected<unordered_map<uint32_t, string>, string> err_input{
        std::unexpected(string::from_view("bad input"))};
    auto err_result = to_test::ResultRoundtrip(std::move(err_input));
    assert(!err_result.has_value());
    assert(str_eq(err_result.error(), "bad input"));
}

static void test_tuple_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> entries{{7, "seven"}};
    auto result = to_test::TupleRoundtrip(std::make_tuple(
        std::span<std::pair<uint32_t, std::string_view> const>(entries), uint64_t(42)));
    auto& [values, number] = result;
    assert(values.size() == 1);
    auto seven = find(values, 7);
    assert(seven && str_eq(*seven, "seven"));
    assert(number == 42);
}

static void test_single_entry_roundtrip() {
    std::vector<std::pair<uint32_t, std::string_view>> input{{99, "ninety-nine"}};
    auto result = to_test::SingleEntryRoundtrip(input);
    assert(result.size() == 1);
    auto value = find(result, 99);
    assert(value && str_eq(*value, "ninety-nine"));
}

void exports::runner::Run() {
    test_named_roundtrip();
    test_bytes_roundtrip();
    test_empty_roundtrip();
    test_option_roundtrip();
    test_record_roundtrip();
    test_inline_roundtrip();
    test_large_roundtrip();
    test_multi_param_roundtrip();
    test_nested_roundtrip();
    test_variant_roundtrip();
    test_result_roundtrip();
    test_tuple_roundtrip();
    test_single_entry_roundtrip();
}
