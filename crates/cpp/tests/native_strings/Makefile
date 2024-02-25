CXXFLAGS=-g -O0 -I../../helper-types

all: libstrings.so app-strings

libstrings.so: the_world.pie.o guest.pie.o
	$(CXX) $(CXXFLAGS) -shared -o $@ $^ -Wl,--version-script=guest.lds
	
%.pie.o: %.cpp
	$(CXX) $(CXXFLAGS) -fPIE -o $@ -c $^

app-strings: the_world_native.o main.o
	$(CXX) $(CXXFLAGS) -o $@ $^ -lstrings
