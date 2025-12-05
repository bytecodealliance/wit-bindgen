//@ args = '--rename a=rename3 --rename foo:bar/b=rename4'

#include <runner.h>

void exports_runner_run() {
    rename3_f();
    rename4_f();
}
