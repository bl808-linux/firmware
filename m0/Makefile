CROSS_COMPILE ?= riscv64-unknown-elf-
CC = $(CROSS_COMPILE)gcc
OBJCOPY = $(CROSS_COMPILE)objcopy

.PHONY: all clean
all: build/m0.bin

build/m0.elf: start.S link.ld Makefile | build
	$(CC) -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -static \
		-Wl,--build-id=none,--no-relax,-T,link.ld,-Map,build/m0.map \
		-o $@ start.S

build/m0.bin: build/m0.elf
	$(OBJCOPY) -O binary $< $@

build:
	mkdir -p $@

clean:
	rm -rf build
