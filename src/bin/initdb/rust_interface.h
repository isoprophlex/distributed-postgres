#ifndef RUST_INTERFACE_H
#define RUST_INTERFACE_H

typedef struct {
    char* name;
} BindingTest;

BindingTest* NewBindingTest(const char* name);

#endif /* RUST_INTERFACE_H */
