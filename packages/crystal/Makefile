ALEF     ?= ../../../../dsisnero/alef/target/debug/alef
CARGO    ?= cargo
CRYSTAL  ?= crystal
REPO     ?= $(abspath $(dir $(lastword $(MAKEFILE_LIST)))../..)

EXAMPLE  := $(REPO)/packages/crystal/bin/example
FFI_DIR  := $(REPO)/target/release
LINKFLAGS := -L$(FFI_DIR) -Wl,-rpath,$(FFI_DIR)

.PHONY: all ffi example run clean

all: example

$(FFI_DIR)/libliter_llm_ffi.a:
	cd $(REPO) && $(ALEF) generate --lang ffi
	cd $(REPO) && $(CARGO) build -p liter-llm-ffi --release

ffi: $(FFI_DIR)/libliter_llm_ffi.a

$(EXAMPLE): $(FFI_DIR)/libliter_llm_ffi.a
	cd $(REPO) && $(ALEF) generate --lang crystal
	cd $(REPO)/packages/crystal && \
	  $(CRYSTAL) build examples/example.cr -o bin/example \
	    --link-flags="$(LINKFLAGS)"

example: $(EXAMPLE)

run: example
	$(EXAMPLE)

clean:
	rm -f $(EXAMPLE)
	rm -rf $(REPO)/packages/crystal/bin/
