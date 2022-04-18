.PHONY: all dis
all:
	as test.s -o test.o
	objcopy -O binary -j .text test.o test.bin
dis:
	objdump -Mintel -d test.o
