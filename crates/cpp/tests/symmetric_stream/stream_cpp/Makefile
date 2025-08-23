CXXFLAGS=-g -I. -fPIC -I../../../../symmetric_executor/cpp-client -I../../../helper-types
LDFLAGS=-L../../../../symmetric_executor/cpp-client \
	-L../../../../symmetric_executor/target/debug \
	-L../target/debug/deps

libstream.so: stream_world.o impl.o
	$(CXX) -shared -o $@ $^ $(LDFLAGS) -lruntime -lsource -lsymmetric_executor -lsymmetric_stream

clean:
	-rm libstream.so stream_world.o impl.o

run: libstream.so
	LD_LIBRARY_PATH=.:../target/debug/deps ../target/debug/main
