//@ args = '--rename a=rename3 --rename foo:bar/b=rename4'

#include <runner.h>

int main() {
    rename3_f();
    rename4_f();
    return 0;
}
