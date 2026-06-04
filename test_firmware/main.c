/* Minimal main — infinite loop */
int main(void) {
    /* Just loop forever to verify flash works */
    while (1) {
        __asm__("nop");
    }
}
