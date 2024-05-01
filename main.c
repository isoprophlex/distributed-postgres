#include <stdio.h>

extern void rust_function();

int main() {
    rust_function();
    printf("Called Rust function");
    return 0;
}
