#include <test_cpp.h>

namespace test_exports = ::exports::test::maps::to_test;

using wit::string;
using wit::unordered_map;
using wit::vector;

static unordered_map<string, uint32_t> invert(unordered_map<uint32_t, string> a) {
    auto result = unordered_map<string, uint32_t>::allocate(a.size());
    size_t i = 0;
    for (auto& [id, name] : a) {
        result.initialize(i++, std::make_pair(std::move(name), id));
    }
    return result;
}

unordered_map<string, uint32_t> test_exports::NamedRoundtrip(unordered_map<uint32_t, string> a) {
    return invert(std::move(a));
}

unordered_map<string, vector<uint8_t>> test_exports::BytesRoundtrip(
    unordered_map<string, vector<uint8_t>> a) {
    return a;
}

unordered_map<uint32_t, string> test_exports::EmptyRoundtrip(unordered_map<uint32_t, string> a) {
    return a;
}

unordered_map<string, std::optional<uint32_t>> test_exports::OptionRoundtrip(
    unordered_map<string, std::optional<uint32_t>> a) {
    return a;
}

test_exports::LabeledEntry test_exports::RecordRoundtrip(test_exports::LabeledEntry a) {
    return a;
}

unordered_map<string, uint32_t> test_exports::InlineRoundtrip(unordered_map<uint32_t, string> a) {
    return invert(std::move(a));
}

unordered_map<uint32_t, string> test_exports::LargeRoundtrip(unordered_map<uint32_t, string> a) {
    return a;
}

std::tuple<unordered_map<string, uint32_t>, unordered_map<string, vector<uint8_t>>>
test_exports::MultiParamRoundtrip(unordered_map<uint32_t, string> a,
                                  unordered_map<string, vector<uint8_t>> b) {
    return std::make_tuple(invert(std::move(a)), std::move(b));
}

unordered_map<string, unordered_map<uint32_t, string>> test_exports::NestedRoundtrip(
    unordered_map<string, unordered_map<uint32_t, string>> a) {
    return a;
}

test_exports::MapOrString test_exports::VariantRoundtrip(test_exports::MapOrString a) {
    return a;
}

std::expected<unordered_map<uint32_t, string>, string> test_exports::ResultRoundtrip(
    std::expected<unordered_map<uint32_t, string>, string> a) {
    return a;
}

std::tuple<unordered_map<uint32_t, string>, uint64_t> test_exports::TupleRoundtrip(
    std::tuple<unordered_map<uint32_t, string>, uint64_t> a) {
    return a;
}

unordered_map<uint32_t, string> test_exports::SingleEntryRoundtrip(
    unordered_map<uint32_t, string> a) {
    return a;
}
