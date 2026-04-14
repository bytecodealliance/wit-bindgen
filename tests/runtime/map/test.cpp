#include <assert.h>
#include <test_cpp.h>

namespace test_exports = ::exports::test::maps::to_test;

std::map<wit::string, uint32_t> test_exports::NamedRoundtrip(std::map<uint32_t, wit::string> a) {
    std::map<wit::string, uint32_t> result;
    for (auto&& [id, name] : a) {
        result.insert(std::make_pair(std::move(name), id));
    }
    return result;
}

std::map<wit::string, wit::vector<uint8_t>> test_exports::BytesRoundtrip(std::map<wit::string, wit::vector<uint8_t>> a) {
    return a;
}

std::map<uint32_t, wit::string> test_exports::EmptyRoundtrip(std::map<uint32_t, wit::string> a) {
    return a;
}

std::map<wit::string, std::optional<uint32_t>> test_exports::OptionRoundtrip(std::map<wit::string, std::optional<uint32_t>> a) {
    return a;
}

test_exports::LabeledEntry test_exports::RecordRoundtrip(test_exports::LabeledEntry a) {
    return a;
}

std::map<wit::string, uint32_t> test_exports::InlineRoundtrip(std::map<uint32_t, wit::string> a) {
    std::map<wit::string, uint32_t> result;
    for (auto&& [k, v] : a) {
        result.insert(std::make_pair(std::move(v), k));
    }
    return result;
}

std::map<uint32_t, wit::string> test_exports::LargeRoundtrip(std::map<uint32_t, wit::string> a) {
    return a;
}

std::tuple<std::map<wit::string, uint32_t>, std::map<wit::string, wit::vector<uint8_t>>> test_exports::MultiParamRoundtrip(std::map<uint32_t, wit::string> a, std::map<wit::string, wit::vector<uint8_t>> b) {
    std::map<wit::string, uint32_t> ids;
    for (auto&& [id, name] : a) {
        ids.insert(std::make_pair(std::move(name), id));
    }
    return std::make_tuple(std::move(ids), std::move(b));
}

std::map<wit::string, std::map<uint32_t, wit::string>> test_exports::NestedRoundtrip(std::map<wit::string, std::map<uint32_t, wit::string>> a) {
    return a;
}

test_exports::MapOrString test_exports::VariantRoundtrip(test_exports::MapOrString a) {
    return a;
}

std::expected<std::map<uint32_t, wit::string>, wit::string> test_exports::ResultRoundtrip(std::expected<std::map<uint32_t, wit::string>, wit::string> a) {
    return a;
}

std::tuple<std::map<uint32_t, wit::string>, uint64_t> test_exports::TupleRoundtrip(std::tuple<std::map<uint32_t, wit::string>, uint64_t> a) {
    return a;
}

std::map<uint32_t, wit::string> test_exports::SingleEntryRoundtrip(std::map<uint32_t, wit::string> a) {
    return a;
}
