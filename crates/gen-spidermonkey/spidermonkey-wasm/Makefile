SM_REPO := https://github.com/fitzgen/gecko-dev
SM_COMMIT := dafd3165f45c55023ece4787a86444029e4f475e

# TODO: support building `spidermonkey.wasm` on other OSes. But for some reason
# the resulting `.wasm` binary is slower when the host compiler is on macOS.
WASI_SDK_URL := https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-12/wasi-sdk-12.0-linux.tar.gz

CC := $(CURDIR)/wasi-sdk-12.0/bin/clang
CXX := $(CURDIR)/wasi-sdk-12.0/bin/clang++

# Set this to `1` to enable logging via all the `SMW_LOG(...)` calls.
LOGGING := 0

# Set this to `-DDEBUG` and uncomment the `--enable-debug` line in `mozconfig`
# to enable debug builds of SpiderMonkey.
DEBUG := ""

# Set this to `""` in debug mode for better debugging.
OPT := "-O2"

CFLAGS := \
	--sysroot=$(CURDIR)/wasi-sdk-12.0/share/wasi-sysroot \
	-Wall \
	--target=wasm32-unknown-wasi \
	-Ispidermonkey-$(SM_COMMIT)/obj-wasm32-unknown-wasi/dist/include \
	-I$(CURDIR)/include \
	$(DEBUG) \
	$(OPT) \
	-DLOGGING=$(LOGGING)

CXXFLAGS := \
	$(CFLAGS) \
	-fno-exceptions \
	-std=c++17

# Local object files.
LOCAL_OBJECTS := $(patsubst %.cpp,%.o,$(wildcard *.cpp))

# Object files needed within SpiderMonkey's obj dir.
SM_OBJECTS := \
	js/src/build/libjs_static.a \
	memory/build/Unified_cpp_memory_build0.o \
	memory/mozalloc/mozalloc_abort.o \
	memory/mozalloc/Unified_cpp_memory_mozalloc0.o \
	mfbt/Unified_cpp_mfbt0.o \
	mfbt/Unified_cpp_mfbt1.o \
	mfbt/lz4.o \
	mfbt/lz4frame.o \
	mfbt/lz4hc.o \
	mfbt/xxhash.o \
	mozglue/misc/AutoProfilerLabel.o \
	mozglue/misc/ConditionVariable_noop.o \
	mozglue/misc/Decimal.o \
	mozglue/misc/MmapFaultHandler.o \
	mozglue/misc/Mutex_noop.o \
	mozglue/misc/Printf.o \
	mozglue/misc/StackWalk.o \
	mozglue/misc/TimeStamp.o \
	mozglue/misc/TimeStamp_posix.o \
	mozglue/misc/Uptime.o \
	modules/zlib/src/compress.o \
	modules/zlib/src/gzclose.o \
	modules/zlib/src/infback.o \
	modules/zlib/src/uncompr.o \
	wasm32-wasi/release/libjsrust.a

# The `./lib/*` copies of SpiderMonkey's object files that we check into the
# repo.
SM_LIB_OBJECTS := $(shell echo $(SM_OBJECTS) | xargs -d' ' -I{} basename {} | xargs -I{} echo lib/{})

.PHONY: all clean clean-all clean-spidermonkey clean-wasi-sdk

all: spidermonkey.wasm
	@echo "Done!"

spidermonkey.initial.wasm: $(SM_LIB_OBJECTS) $(LOCAL_OBJECTS)
	$(CXX) $(CXXFLAGS) \
		-mexec-model=reactor \
		$(LOCAL_OBJECTS) \
		$(SM_LIB_OBJECTS) \
		-o spidermonkey.initial.wasm \
		-Wl,--export-dynamic \
		-Wl,--growable-table \
		-Wl,--export-table \
		-Wl,--gc-sections

spidermonkey.wasm: spidermonkey.initial.wasm
	# Uncomment this `wasm-opt` invocation and comment the following one out to
	# enable better debugging.
	#
	# wasm-opt -g --duplicate-import-elimination spidermonkey.initial.wasm -o spidermonkey.wasm
	wasm-opt -O2 --strip-dwarf --duplicate-import-elimination spidermonkey.initial.wasm -o spidermonkey.wasm


# Build all `*.cpp` files into `*.o` files.
%.o: %.cpp $(SM_LIB_OBJECTS)
	$(CXX) $(CXXFLAGS) -c $< -o $@

# Actually build SpiderMonkey.
$(SM_LIB_OBJECTS): spidermonkey-$(SM_COMMIT) mozbuild wasi-sdk-12.0 mozconfig
	cd spidermonkey-$(SM_COMMIT) \
		&& MOZBUILD_STATE_PATH=$(CURDIR)/mozbuild MOZCONFIG=$(CURDIR)/mozconfig ./mach build
	mkdir -p lib
	for x in $(SM_OBJECTS); do \
	    cp spidermonkey-$(SM_COMMIT)/obj-wasm32-unknown-wasi/$$x lib/; \
	done

# Clone `mozilla-central` at the `SM_COMMIT` commit.
spidermonkey-$(SM_COMMIT):
	-rm -rf spidermonkey-temp
	mkdir spidermonkey-temp
	cd spidermonkey-temp \
		&& git init \
		&& git remote add origin $(SM_REPO) \
		&& git fetch origin $(SM_COMMIT) \
		&& git checkout $(SM_COMMIT)
	mv spidermonkey-temp spidermonkey-$(SM_COMMIT)

mozbuild: spidermonkey-$(SM_COMMIT)
	-mkdir mozbuild
	cd spidermonkey-$(SM_COMMIT) \
		&& MOZBUILD_STATE_PATH=$(CURDIR)/mozbuild ./mach bootstrap --application-choice js --no-system-changes \
		|| rm -rf $(CURDIR)/mozbuild

wasi-sdk-12.0:
	curl -L $(WASI_SDK_URL) | tar -x -z

clean-all: clean clean-spidermonkey clean-wasi-sdk

clean-wasi-sdk:
	-rm -rf wasi-sdk-12.0

clean-spidermonkey:
	-rm -rf spidermonkey-$(SM_COMMIT)
	-rm -rf spidermonkey-$(SM_COMMIT)/obj-wasm32-unknown-wasi/
	-rm -rf mozbuild

clean:
	@echo 'Only cleaning our own artifacts, not upstream deps. Run `make clean-{all,spidermonkey,wasi-sdk}` to clean others.'
	-rm -rf spidermonkey-temp
	-rm -rf ./*.o
	-rm -rf spidermonkey.wasm
