TARGET     := riscv64gc-unknown-none-elf
MODE       := release
BIN_NAME   := kernel

TARGET_DIR := target/$(TARGET)/$(MODE)
KERNEL     := $(TARGET_DIR)/$(BIN_NAME)

QEMU       := qemu-system-riscv64
QEMU_OPTS  := -machine virt \
              -nographic \
			  -bios default \
              -kernel $(KERNEL)

.PHONY: qemu build strip clean

qemu: build
	$(QEMU) $(QEMU_OPTS)

build:
	cargo build --release

clean:
	cargo clean
