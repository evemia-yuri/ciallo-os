TARGET     := riscv64gc-unknown-none-elf
MODE       := release
BIN_NAME   := kernel

BOOTLOADER := bootloader/rustsbi-qemu.bin
TARGET_DIR := target/$(TARGET)/$(MODE)
KERNEL_BIN := $(TARGET_DIR)/$(BIN_NAME).bin

QEMU       := qemu-system-riscv64
QEMU_OPTS  := -machine virt \
              -nographic \
              -bios $(BOOTLOADER) \
              -device loader,file=$(KERNEL_BIN),addr=0x80200000

.PHONY: qemu build strip clean

qemu: build strip
	$(QEMU) $(QEMU_OPTS)

build:
	cargo build --release

strip:
	cargo objcopy \
		      --release \
		      --package $(BIN_NAME) \
		      --bin $(BIN_NAME) \
		      -- -O binary $(KERNEL_BIN)

clean:
	cargo clean
