CROSS_COMPILE ?= riscv64-unknown-elf-
CC = $(CROSS_COMPILE)gcc
OBJCOPY = $(CROSS_COMPILE)objcopy
BUILD_DIR = $(if $(O),$(O),build)

.PHONY: all clean
all: $(BUILD_DIR)/m0.bin

$(BUILD_DIR)/m0.elf: start.S link.ld Makefile | $(BUILD_DIR)
	$(CC) -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -static \
		-Wl,--build-id=none,--no-relax,-T,link.ld,-Map,$(BUILD_DIR)/m0.map \
		-o $@ start.S

$(BUILD_DIR)/m0.bin: $(BUILD_DIR)/m0.elf
	$(OBJCOPY) -O binary $< $@

$(BUILD_DIR):
	mkdir -p $@

clean:
	rm -rf $(BUILD_DIR)
