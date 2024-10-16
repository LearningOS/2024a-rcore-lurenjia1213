    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top #配置栈，然后跳转到rust_main
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    #16K的启动栈
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top: