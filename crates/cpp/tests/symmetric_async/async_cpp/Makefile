CXXFLAGS=-g -Isrc -fPIC
LDFLAGS=-L../../../../symmetric_executor/cpp-client \
	-L../../../../symmetric_executor/target/debug \
	-L../target/debug/deps

libasync_module.so: async_module.o middle.o
	$(CXX) -shared -o $@ $^ $(LDFLAGS) -lruntime -lsleep -lsymmetric_executor -lsymmetric_stream

%.o: src/%.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $^

clean:
	-rm libasync_module.so async_module.o middle.o

run: libasync_module.so
	LD_LIBRARY_PATH=.:../target/debug/deps ../target/debug/main
