    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top #配置栈，然后跳转到rust_main
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    #64K的启动栈
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
#栈顶地址被全局符号 boot_stack_top 标识(sp寄存器)，栈底则被全局符号 boot_stack 标识。