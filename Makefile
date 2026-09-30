TARGET     := riscv64gc-unknown-none-elf
MODE       := release

TARGET_DIR := target/$(TARGET)/$(MODE)
KERNEL_ELF := $(TARGET_DIR)/kernel

QEMU       := qemu-system-riscv64
QEMU_OPTS  := -machine virt \
              -nographic \
	      -bios default \
              -kernel $(KERNEL_ELF)

qemu: build
	$(QEMU) $(QEMU_OPTS)

qemu-gdb: build
	$(QEMU) $(QEMU_OPTS) -s -S

build:
	cargo build --release

.PHONY: clean format

clean:
	cargo clean

format:
	cargo fmt
