PKG_NAME=colerr
DOCS_DEFAULT_MODULE=colerr
DEFAULT_TARGET=build
EXAMPLES =

default: $(DEFAULT_TARGET)

# Mostly generic part goes below

ifneq ($(RELEASE),)
$(info RELEASE BUILD)
CARGO_FLAGS += --release
ALL_TARGETS += build test bench $(EXAMPLES)
else
$(info DEBUG BUILD; use `RELEASE=true make [args]` for release build)
ALL_TARGETS += build test $(EXAMPLES)
endif

all: $(ALL_TARGETS)

.PHONY: run test build doc clean
run test build clean:
	cargo $@ $(CARGO_FLAGS)

.PHONY: bench
bench:
	cargo $@ $(filter-out --release,$(CARGO_FLAGS))

.PHONY: longtest
longtest:
	for i in `seq 100`; do cargo test $(CARGO_FLAGS) || exit 1 ; done

.PHONY: $(EXAMPLES)
$(EXAMPLES):
	cargo build --example $@ $(CARGO_FLAGS)

.PHONY: doc
doc:
	cargo doc

.PHONY: publishdoc
publishdoc: doc
	echo '<meta http-equiv="refresh" content="0;url='${DOCS_DEFAULT_MODULE}'/index.html">' > target/doc/index.html
	ghp-import -n target/doc
	git push -f origin gh-pages

.PHONY: docview
docview: doc
	xdg-open target/doc/$(PKG_NAME)/index.html

.PHONY: FORCE
FORCE:
