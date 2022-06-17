.intel_syntax noprefix


.rept 32
	nop
.endr

_start:
	mov rax, 0
	mov rbx, 0
	mov rcx, 0
	mov rdx, 0
	mov rdi, 0
	mov rsi, 0
	mov rsp, 0x10000
	jmp main
.align 64
main:
	mov rax, 0x10
	mov rbx, 0x20
	add rax, rbx
	mov [rsp+0x10], rax
	mov [rsp+0x20], rax
	add rax, rbx
	mov [rsp+0x30], rax
	mov [rsp+0x40], rax
	ud2
.align 64
