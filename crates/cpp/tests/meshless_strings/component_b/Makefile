CXXFLAGS=-g -O0 -I../../../helper-types

all: libcomponent_b.a

libcomponent_b.a: the_world.o guest.o
	ar rcvs $@ $^

bindgen:
	../../../../../target/debug/wit-bindgen cpp ../wit -w b --wasm64 --format

clean:
	-rm *~ *.a *.o
