## ESP32-S3 (Xtensa) workflow helpers
## Usage examples:
##   make build              # Build (release by default)
##   make run PORT=/dev/tty.usbmodem1101 BAUD=115200
##   make attach PORT=/dev/tty.usbmodem1101 BAUD=115200
##   make ports              # List serial ports detected by espflash
##   make env                # Show resolved variables

# -------- Configuration (override via environment) --------
TARGET  ?= xtensa-esp32s3-none-elf
# Binary name from Cargo.toml [package].name
BIN     ?= esp32s3-hello-world
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

# Compose espflash flags
PORT_FLAG   := $(if $(PORT),--port $(PORT),)
BAUD_FLAG   := -B $(BAUD)
CHIP_FLAG   := --chip $(CHIP)
LOGFMT_FLAG := --log-format $(LOGFMT)
ESPFLASH_ARGS ?=

.PHONY: help env ports build clean run attach monitor flash

help:
	@echo "Makefile targets:"
	@echo "  make build                 Build firmware ($(PROFILE))"
	@echo "  make run   [PORT=/dev/ttyX] [BAUD=115200]  Flash and monitor with defmt decode"
	@echo "  make attach[PORT=/dev/ttyX] [BAUD=115200]  Attach monitor to existing firmware (defmt)"
	@echo "  make ports                 List serial ports detected by espflash"
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
	espflash scan-ports || espflash list-ports || true

# Ensure the xtensa target is honored (also set in .cargo/config.toml)
build:
	cargo build $(CARGO_FLAGS)

clean:
	cargo clean

# Flash and then monitor (defmt decoded). Works even if monitor/attach is run separately.
run: build
	espflash flash $(ELF) --monitor $(PORT_FLAG) $(BAUD_FLAG) $(CHIP_FLAG) $(LOGFMT_FLAG) $(ESPFLASH_ARGS)

# Attach only to serial monitor with defmt decoding.
# Requires a recent build so that $(ELF) exists for defmt symbol decoding.
attach: $(ELF)
	espflash monitor $(PORT_FLAG) $(BAUD_FLAG) $(CHIP_FLAG) $(LOGFMT_FLAG) --elf $(ELF) $(ESPFLASH_ARGS)

# Alias: monitor (same as attach)
monitor: attach
