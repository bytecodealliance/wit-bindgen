CXXFLAGS=-g -O0 -I../../../helper-types

all: component_a

component_a: a.cpp main.cpp
	$(CXX) -shared $(CXXFLAGS) -o $@ $^

bindgen:
	../../../../../target/debug/wit-bindgen cpp ../wit -w a --wasm64 --format
