cargo build

qemu-system-riscv32 \
    -monitor stdio \
    -machine virt \
    -bios default \
    --no-reboot \
    -kernel ./target/riscv32i-unknown-none-elf/debug/os1k
