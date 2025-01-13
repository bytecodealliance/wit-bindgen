CXXFLAGS=-I../../cpp/helper-types -g -fPIC

all: libruntime.a

libruntime.a: module.o
	${AR} rcuvs $@ $^

clean:
	-rm module.o libruntime.a
