TARGET      := riscv64gc-unknown-none-elf
MODE        := release

TARGET_DIR  := target/$(TARGET)/$(MODE)
KERNEL_ELF  := $(TARGET_DIR)/kernel

QEMU        := qemu-system-riscv64
QEMU_OPTS   := -machine virt \
              -nographic \
              -bios default

TEST_RUNNER := $(QEMU) $(QEMU_OPTS) -kernel

.PHONY: qemu qemu-gdb build test clean format

qemu: build
	$(QEMU) $(QEMU_OPTS) -kernel $(KERNEL_ELF)

qemu-gdb: build
	$(QEMU) $(QEMU_OPTS) -kernel $(KERNEL_ELF) -s -S

build:
	cargo build --release

test:
	CARGO_TARGET_RISCV64GC_UNKNOWN_NONE_ELF_RUNNER="$(TEST_RUNNER)" cargo test

clean:
	cargo clean

format:
	cargo fmt
