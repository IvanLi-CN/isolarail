## ESP32-S3 (Xtensa) compatibility helpers
## Usage examples:
##   make build              # Build (release by default)
##   make run                # Delegate to just flash-monitor
##   make attach             # Delegate to just monitor
##   make ports              # List Local USB candidates through isohub/devd
##   make env                # Show resolved variables

# -------- Configuration (override via environment) --------
TARGET  ?= xtensa-esp32s3-none-elf
# Binary name from Cargo.toml [package].name
BIN     ?= iso-usb-hub
# Profile: release or debug
PROFILE ?= release
# Serial port (optional). Example on macOS: /dev/tty.usbmodem1101 or /dev/tty.usbserial-xxxx
PORT    ?=
# Baud rate for serial monitor
BAUD    ?= 115200
# Chip type for espflash
CHIP    ?= esp32s3
# Log format for espflash monitor (defmt required to decode logs)
LOGFMT  ?= defmt

# -------- Derived paths --------
ifeq ($(PROFILE),release)
  CARGO_FLAGS := --release
else
  CARGO_FLAGS :=
endif

BINDIR := target/$(TARGET)/$(PROFILE)
ELF    := $(BINDIR)/$(BIN)

# Legacy variables retained for compatibility with older invocations.
PORT_FLAG   := $(if $(PORT),--port $(PORT),)
BAUD_FLAG   := -B $(BAUD)
CHIP_FLAG   := --chip $(CHIP)
LOGFMT_FLAG := --log-format $(LOGFMT)
ESPFLASH_ARGS ?=

.PHONY: help env ports build clean run attach monitor flash

help:
	@echo "Makefile targets:"
	@echo "  make build                 Build firmware ($(PROFILE))"
	@echo "  make run                   Delegate to just flash-monitor"
	@echo "  make attach                Delegate to just monitor"
	@echo "  make ports                 List Local USB candidates through isohub/devd"
	@echo "  make env                   Show resolved variables"
	@echo "Variables (override via env): TARGET BIN PROFILE PORT BAUD CHIP LOGFMT"

env:
	@echo TARGET = $(TARGET)
	@echo BIN    = $(BIN)
	@echo PROFILE= $(PROFILE)
	@echo BINDIR = $(BINDIR)
	@echo ELF    = $(ELF)
	@echo PORT   = $(PORT)
	@echo BAUD   = $(BAUD)
	@echo CHIP   = $(CHIP)
	@echo LOGFMT = $(LOGFMT)

ports:
	just ports

# Ensure the xtensa target is honored (also set in .cargo/config.toml)
build:
	cargo +esp build $(CARGO_FLAGS)

clean:
	cargo clean

# Flash and then monitor through the Local USB CLI/devd path.
run: build
	just flash-monitor

# Attach only to serial monitor through the Local USB CLI/devd path.
# Requires a recent build so that $(ELF) exists for defmt symbol decoding.
attach: $(ELF)
	just monitor

# Alias: monitor (same as attach)
monitor: attach
