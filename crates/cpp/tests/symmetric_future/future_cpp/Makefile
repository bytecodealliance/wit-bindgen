CXXFLAGS=-g -I. -fPIC -I../../../../symmetric_executor/cpp-client -I../../../helper-types
LDFLAGS=-L../../../../symmetric_executor/cpp-client \
	-L../../../../symmetric_executor/target/debug \
	-L../target/debug/deps

libfuture.so: future_world.o future.o
	$(CXX) -shared -o $@ $^ $(LDFLAGS) -lruntime -lsource -lsymmetric_executor -lsymmetric_stream

clean:
	-rm libfuture.so future_world.o future.o

run: libfuture.so
	LD_LIBRARY_PATH=.:../target/debug/deps ../target/debug/main
