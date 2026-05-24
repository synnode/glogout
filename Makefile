# glogout — build & install helpers.
#
# Default install is user-local (~/.cargo/bin, no sudo), matching the bundled
# systemd unit's ExecStart. For a system-wide install:
#   make build && sudo make install PREFIX=/usr/local
# Packagers can stage into a DESTDIR (build first, then install):
#   make build && make install DESTDIR=pkg PREFIX=/usr

PREFIX  ?= $(HOME)/.cargo
DESTDIR ?=
BINDIR  := $(DESTDIR)$(PREFIX)/bin
# systemd *user* unit dir. Override for a system install, e.g.
#   UNITDIR=$(DESTDIR)/usr/lib/systemd/user
UNITDIR ?= $(DESTDIR)$(HOME)/.config/systemd/user

BIN := target/release/glogout

.PHONY: all build install uninstall run clean help

all: build

## build: compile the release binary
build:
	cargo build --release

## install: build, then install the binary + systemd user unit
install: build
	install -Dm755 $(BIN) $(BINDIR)/glogout
	install -Dm644 contrib/glogout.service $(UNITDIR)/glogout.service
	@echo "→ installed $(BINDIR)/glogout"
	@echo "→ installed $(UNITDIR)/glogout.service  (enable: systemctl --user enable --now glogout)"
	@resolved="$$(command -v glogout 2>/dev/null)"; \
	if [ -n "$$resolved" ] && [ "$$resolved" != "$(BINDIR)/glogout" ]; then \
		printf '\n\033[33m⚠  PATH resolves "glogout" to %s\033[0m\n' "$$resolved"; \
		echo "   The copy you just installed at $(BINDIR)/glogout is shadowed."; \
		echo "   Remove the other one (e.g. sudo rm $$resolved) or put $(BINDIR) earlier in PATH."; \
	fi

## uninstall: remove the binary and systemd user unit
uninstall:
	rm -f $(BINDIR)/glogout $(UNITDIR)/glogout.service
	@echo "→ removed $(BINDIR)/glogout"

## run: build and run from source (pass args via ARGS, e.g. make run ARGS=daemon)
run:
	cargo run --release -- $(ARGS)

## clean: cargo clean
clean:
	cargo clean

## help: list targets
help:
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/^## /  /'
