[BITS 16]
[org 0x7c00]

mov [BOOT_DEVICE], dl

; setup stack
mov bp, 0x9000
mov sp, bp

; Check if Vesa mode is available
mov  ax, 0x4F01
mov  cx, 0x0118
mov  bx, 0x0800
mov  es, bx
mov  di, 0x00
int  0x10
mov  al, [es:di]
test al, al
jns  NoLFB
pop es

; Enable Vesa mode
mov  ax, 0x4F02      ; What VBA service is wanted?
mov  bx, 0x0118
int  0x10

call load_kernel					; Load entire kernel into memory
call switch_to_32bit			; switch to 32 bit mode and enable A20

NoLFB:
		; uh oh
		hlt

load_kernel:
		mov bx, 0x1000        ; Destination address
		mov dh, 16            ;                                     <-- IF THERE'S PROBLEMS IT'S PROBABLY HERE!!
		mov dl, [BOOT_DEVICE] ; Drive Number

		mov al, dh
		mov ch, 0x00 ; Cylinder number
		mov cl, 0x02 ; Start from sector 2, sector 1 is our bootloader
		mov ah, 0x02 ; Set read mode 
		mov dh, 0x00

		int 0x13
		ret

gdtp:
		dw gdt_end - gdt_start - 1
		dw gdt_start

gdt_start:
		dq 0x0
gdt_code_segment:
		dw 0xFFFF         ; Limit (16-bit) (set to 0xFFFF for maximum)
		dw 0x0            ; Base (16-bit)
		db 0x0            ; Base (8-bit)
		db 10011010b      ; Access flags
		db 11001111b      ; Granularity flags (set to 0xCF for 4 KB granularity)
		db 0x0            ; Segment bits
gdt_data_segment:
		dw 0xFFFF         ; Limit (16-bit) (set to 0xFFFF for maximum)
		dw 0x0            ; Base (16-bit)
		db 0x0            ; Base (8-bit)
		db 10010010b      ; Access flags
		db 11001111b      ; Granularity flags (set to 0xCF for 4 KB granularity)
		db 0x0            ; Segment bits
gdt_end:

GDT_CODE_SEG_ADDR equ gdt_code_segment - gdt_start
GDT_DATA_SEG_ADDR equ gdt_data_segment - gdt_start

switch_to_32bit:
		; Disable interrupts and setup gdt 
		cli
		lgdt [gdtp]

		; Enable A20 with Fast A20 Gate
		in al, 0x92
		or al, 2
		out 0x92, al

		; Set PE (Protection Enable) bit in CR0
		mov eax, cr0
		or eax, 0x01
		mov cr0, eax
		jmp GDT_CODE_SEG_ADDR:init_32bit

[BITS 32]
init_32bit:
		mov ax, GDT_DATA_SEG_ADDR
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

		mov ebp, 0x90000
		mov esp, ebp

		call init_kernel

init_kernel:
		call 0x1000
		jmp $

BOOT_DEVICE db 0x00

times 510 - ($ - $$) db 0 ; make sure file is 510 bytes in size
dw 0xAA55                 ; write boot signature