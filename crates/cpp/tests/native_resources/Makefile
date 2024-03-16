CXXFLAGS=-g -O0 -I../../helper-types
WIT_BINDGEN=../../../../target/debug/wit-bindgen

all: libresources.so app-resources

libresources.so: the_world.pie.o guest.pie.o
	$(CXX) $(CXXFLAGS) -shared -o $@ $^ -Wl,--version-script=guest.lds
	
%.pie.o: %.cpp
	$(CXX) $(CXXFLAGS) -fPIE -o $@ -c $^

app-resources: the_world_native.o main.o
	$(CXX) $(CXXFLAGS) -o $@ $^ -L. -lresources

bindgen: wit/resources_simple.wit
	$(WIT_BINDGEN) cpp wit --wasm64 --format
	$(WIT_BINDGEN) cpp wit --wasm64 --format --direct
	cd rust/src ; ../../$(WIT_BINDGEN) rust ../../wit --wasm64

guest.wasm: the_world.cpp guest.cpp
	/opt/wasi-sdk/bin/clang++ -o $@ $^ $(CXXFLAGS) 
	
clean:
	-rm *.o libresources.so app-resources
