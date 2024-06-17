#include <stdio.h>

extern int rust_function(int number);

int main() {
    int response = rust_function(32);
    printf("Called Rust function, returned: %d\n", response);
    return 15;
}