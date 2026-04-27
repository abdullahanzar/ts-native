#include <stdint.h>
#include <stdio.h>

void __tsn_print_int(int64_t value) {
    printf("%lld\n", (long long)value);
}

void __tsn_print_double(double value) {
    printf("%g\n", value);
}

void __tsn_print_bool(int64_t value) {
    puts(value ? "true" : "false");
}