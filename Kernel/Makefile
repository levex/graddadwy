
# CONFIG: Architecture to build for
ARCH ?= amd64

ifeq ($(ARCH),amd64)
    TRIPLE ?= x86_64-none-elf-
else ifeq ($(ARCH),x86)
    TRIPLE ?= i686-elf-
else
    $(error Unknown architecture $(ARCH))
endif

MEM ?= 128

# Toolchain commands (can be overridden)
RUSTC ?= rustc
LD := $(TRIPLE)ld
AS := $(TRIPLE)as
OBJDUMP := $(TRIPLE)objdump
OBJCOPY := $(TRIPLE)objcopy

# Object directory
OBJDIR := .obj/$(ARCH)/

LINKSCRIPT := arch/$(ARCH)/link.ld
TARGETSPEC := arch/$(ARCH)/target.json
# Compiler Options
LINKFLAGS := -T $(LINKSCRIPT)
#LINKFLAGS += -Map $(OBJDIR)map.txt
LINKFLAGS += --gc-sections
LINKFLAGS += -z max-page-size=0x1000

# Objects
LIBCORE := $(OBJDIR)libcore.rlib
OBJS := start.o kernel.o libcore.rlib libcompiler_builtins.rlib
OBJS := $(OBJS:%=$(OBJDIR)%)
BIN := ../kernel.$(ARCH).bin

.PHONY: all clean UPDATE run $(BIN)

all: $(BIN)

$(OBJDIR)start.o: arch/$(ARCH)/start.S Makefile $(LINKSCRIPT)
	@mkdir -p $(dir $@)
	$(AS) $(ASFLAGS) -o $@ $<

$(BIN): $(OBJDIR)start.o
	xargo build --target x86_64-graddadwy
	$(LD) -o $@ $(LINKFLAGS) $(OBJDIR)start.o target/x86_64-graddadwy/debug/libkernel.a
	mv $@ $@.elf64
	$(OBJCOPY) -K SMP_AP_START -K SMP_AP_END -K kernel_end $@.elf64 -F elf32-i386 $@

run: $(BIN)
	qemu-system-x86_64 -cpu max -kernel ../kernel.amd64.bin -smp 2 -serial stdio -nographic -monitor null -m $(MEM)

run_smp: $(BIN)
	qemu-system-x86_64 -cpu max -kernel ../kernel.amd64.bin -smp $(SMP) -serial stdio -nographic -monitor null -m $(MEM)

run_up: $(BIN)
	qemu-system-x86_64 -cpu max -kernel ../kernel.amd64.bin -serial stdio -nographic -monitor null -m $(MEM)
run: $(BIN)
	qemu-system-x86_64 -cpu max -kernel ../kernel.amd64.bin -smp 2 -serial stdio -nographic -monitor null -m $(MEM)

drun: $(BIN)
	qemu-system-x86_64 -d int -cpu max -kernel ../kernel.amd64.bin -smp 2 -serial stdio -nographic -monitor null -m $(MEM)

gdb_run: $(BIN)
	qemu-system-x86_64 -d int -cpu max -kernel ../kernel.amd64.bin -smp 16 -serial stdio -nographic -monitor null -m $(MEM) -s -S

# Include dependency files
-include $(OBJDIR)libcore.d $(OBJDIR)kernel.d $(OBJDIR)start.d
