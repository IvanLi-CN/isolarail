CHIP = STM32G431CBUx
TARGET_DIR = target/thumbv7em-none-eabihf
BIN_NAME = iso-usb-hub

.PHONY: attach attach-release reset reset-release reset-attach reset-attach-release

attach:
	probe-rs attach --chip $(CHIP) $(TARGET_DIR)/debug/${BIN_NAME}

attach-release:
	probe-rs attach --chip $(CHIP) $(TARGET_DIR)/release/${BIN_NAME}

reset:
	probe-rs reset --chip $(CHIP)

reset-release:
	probe-rs reset --chip $(CHIP)

reset-attach: reset
	probe-rs attach --chip $(CHIP) $(TARGET_DIR)/debug/${BIN_NAME}

reset-attach-release: reset-release
	probe-rs attach --chip $(CHIP) $(TARGET_DIR)/release/${BIN_NAME}