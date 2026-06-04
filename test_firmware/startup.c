/* Minimal startup code for STM32F411 */
extern void _estack(void);

void Reset_Handler(void);
void Default_Handler(void) { while (1); }

/* Forward declarations of exception handlers */
void NMI_Handler(void)           __attribute__((weak, alias("Default_Handler")));
void HardFault_Handler(void)     __attribute__((weak, alias("Default_Handler")));
void SVC_Handler(void)           __attribute__((weak, alias("Default_Handler")));
void PendSV_Handler(void)        __attribute__((weak, alias("Default_Handler")));
void SysTick_Handler(void)       __attribute__((weak, alias("Default_Handler")));

/* Vector table */
__attribute__((section(".isr_vector"), used))
void (* const vector_table[])(void) = {
    &_estack,           /* 0x0000: Initial SP */
    Reset_Handler,      /* 0x0004: Reset */
    NMI_Handler,        /* 0x0008: NMI */
    HardFault_Handler,  /* 0x000C: Hard Fault */
    0, 0, 0, 0, 0, 0, 0, /* reserved */
    SVC_Handler,        /* 0x002C: SVCall */
    0, 0,               /* reserved */
    PendSV_Handler,     /* 0x0038: PendSV */
    SysTick_Handler,    /* 0x003C: SysTick */
};

/* External symbols from linker script */
extern unsigned int _sidata;
extern unsigned int _sdata, _edata;
extern unsigned int _sbss, _ebss;

void Reset_Handler(void) {
    /* Copy .data from flash to RAM */
    for (unsigned int *src = &_sidata, *dst = &_sdata; dst < &_edata; src++, dst++)
        *dst = *src;

    /* Zero .bss */
    for (unsigned int *dst = &_sbss; dst < &_ebss; dst++)
        *dst = 0;

    /* Call main */
    extern int main(void);
    main();

    /* If main returns, loop forever */
    while (1);
}
