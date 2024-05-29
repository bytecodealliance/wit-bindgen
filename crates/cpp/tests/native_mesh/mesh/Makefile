CXXFLAGS=-fPIC -g -O0 -I../../../helper-types

all: libmesh.so

libmesh.so: mesh_native.cpp
	$(CXX) -shared $(CXXFLAGS) -o $@ $^

bindgen:
	../../../../../target/debug/wit-bindgen cpp ../wit -w mesh --wasm64 --format --direct --split-interfaces
