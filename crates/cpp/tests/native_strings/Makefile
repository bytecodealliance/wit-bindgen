CXXFLAGS=-g -O0 -I../../helper-types
WIT_BINDGEN=../../../../target/debug/wit-bindgen

all: libstrings.so app-strings

libstrings.so: the_world.pie.o guest.pie.o
	$(CXX) $(CXXFLAGS) -shared -o $@ $^ -Wl,--version-script=guest.lds
	
%.pie.o: %.cpp
	$(CXX) $(CXXFLAGS) -fPIE -o $@ -c $^

app-strings: the_world_native.o main.o
	$(CXX) $(CXXFLAGS) -o $@ $^ -L. -lstrings

bindgen: wit/strings.wit
	$(WIT_BINDGEN) cpp wit --wasm64 --format
	$(WIT_BINDGEN) cpp wit --wasm64 --format --direct

guest.wasm: the_world.cpp guest.cpp
	/opt/wasi-sdk/bin/clang++ -o $@ $^ $(CXXFLAGS) 