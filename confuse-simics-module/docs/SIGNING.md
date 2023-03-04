Write the simics_constants.rs dynamically generated file for inclusion into the module
We basically need to fake this file:

```c
/* module_id.c - automatically generated, do not edit */

#include <simics/build-id.h>
#include <simics/base/types.h>
#include <simics/util/help-macros.h>

#if defined(SIMICS_6_API)
#define BUILD_API "6"
#elif defined(SIMICS_5_API)
#define BUILD_API "5"
#elif defined(SIMICS_4_8_API)
#define BUILD_API "4.8"
#else
#define BUILD_API "?"
#endif

#define EXTRA "                                           "

EXPORTED const char _module_capabilities_[] =
        "VER:" SYMBOL_TO_STRING(SIM_VERSION_COMPAT) ";"
        "ABI:" SYMBOL_TO_STRING(SIM_VERSION) ";"
        "API:" BUILD_API ";"
        "BLD:" "0" ";"
        "BLD_NS:__simics_project__;"
         // date +'%s'
        "BUILDDATE:" "1677199642" ";"
        "MOD:" "afl-branch-tracer" ";"
        "CLS:afl_branch_tracer" ";"
        "HOSTTYPE:" "linux64" ";"
        "THREADSAFE;"
        EXTRA ";";
// date +'%a %b %d %T %Y'
EXPORTED const char _module_date[] = "Fri Feb 24 00:47:22 2023";
EXPORTED void _simics_module_init(void);
extern void sim_iface_wrap_init(void);

void
_simics_module_init(void)
{

        init_local();
}
```

The build process that produces it is:

`/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/cctype.py --type gcc`
`gcc -v`
`gcc -dumpversion`
`rm -rf linux64/obj/modules/`
`ps -ax -o pid=,ppid=,pcpu=,pmem=,command=`
`/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/project_setup.py --project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ --check-project-version`
`/home/rhart/install/simics/simics-6.0.157/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/envcheck.py -c gcc linux64/.environment-check/all`
`/home/rhart/install/simics/simics-6.0.157/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/module_id.py --c-module-id --output module_id.c --module-name afl-branch-tracer --classes afl_branch_tracer; --components  --host-type linux64 --thread-safe yes --user-init-local`
`/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -E -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -M -MF module_id.d -MP -MT module_id.d -MT module_id.o -D HAVE_MODULE_DATE -D SIMICS_6_API module_id.c -mtune=generic -march=x86-64 -std=gnu99 -fvisibility=hidden -fasynchronous-unwind-tables -fstack-protector-strong -Wformat -Wformat-security -fstack-clash-protection -fcf-protection -dumpdir a- -dumpbase module_id.c -dumpbase-ext .c`
`/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -E -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -M -MF afl-branch-tracer.d -MP -MT afl-branch-tracer.d -MT afl-branch-tracer.o -D HAVE_MODULE_DATE -D SIMICS_6_API /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer/afl-branch-tracer.c -mtune=generic -march=x86-64 -std=gnu99 -fvisibility=hidden -fasynchronous-unwind-tables -fstack-protector-strong -Wformat -Wformat-security -fstack-clash-protection -fcf-protection -dumpdir a- -dumpbase afl-branch-tracer.c -dumpbase-ext .c`
`/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -D HAVE_MODULE_DATE -D SIMICS_6_API -D _FORTIFY_SOURCE=2 /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer/afl-branch-tracer.c -quiet -dumpbase afl-branch-tracer.c -dumpbase-ext .c -mtune=generic -march=x86-64 -gdwarf-2 -O2 -Wall -Wwrite-strings -Wformat-security -std=gnu99 -fvisibility=hidden -fPIC -fasynchronous-unwind-tables -fstack-protector-strong -Wformat-security -fstack-clash-protection -fcf-protection -o /tmp/ccadVVa4.s`
`/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -imultiarch x86_64-linux-gnu -D HAVE_MODULE_DATE -D SIMICS_6_API -D _FORTIFY_SOURCE=2 module_id.c -quiet -dumpbase module_id.c -dumpbase-ext .c -mtune=generic -march=x86-64 -gdwarf-2 -O2 -Wall -Wwrite-strings -Wformat-security -std=gnu99 -fPIC -fasynchronous-unwind-tables -fstack-protector-strong -Wformat-security -fstack-clash-protection -fcf-protection -o /tmp/cckZqqtx.s`
`/usr/lib/gcc/x86_64-linux-gnu/11/collect2 -plugin /usr/lib/gcc/x86_64-linux-gnu/11/liblto_plugin.so -plugin-opt=/usr/lib/gcc/x86_64-linux-gnu/11/lto-wrapper -plugin-opt=-fresolution=/tmp/ccvZgDwH.res -plugin-opt=-pass-through=-lgcc_s -plugin-opt=-pass-through=-lc -plugin-opt=-pass-through=-lgcc_s --build-id --eh-frame-hdr -m elf_x86_64 --hash-style=gnu --as-needed -shared -z relro -o /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so -z noexecstack -z relro -z now /usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu/crti.o /usr/lib/gcc/x86_64-linux-gnu/11/crtbeginS.o -L/home/rhart/install/simics/simics-6.0.157/linux64/bin -L/usr/lib/gcc/x86_64-linux-gnu/11 -L/usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu -L/usr/lib/gcc/x86_64-linux-gnu/11/../../../../lib -L/lib/x86_64-linux-gnu -L/lib/../lib -L/usr/lib/x86_64-linux-gnu -L/usr/lib/../lib -L/usr/lib/gcc/x86_64-linux-gnu/11/../../.. --version-script /home/rhart/install/simics/simics-6.0.157/config/project/exportmap.elf afl-branch-tracer.o module_id.o -lsimics-common -lvtutils -lstdc++ -lm -lgcc_s -lc -lgcc_s /usr/lib/gcc/x86_64-linux-gnu/11/crtendS.o /usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu/crtn.o`
`/home/rhart/install/simics/simics-6.0.157/bin/simics -project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ -batch-mode -quiet -no-copyright -no-module-cache -sign-module /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so`
`/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/simics-common -project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ -batch-mode -quiet -no-copyright -no-module-cache -sign-module /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so`

Basically:
- Generate a C file
- Compile against the C file
- "Sign" the module (not sure what that means)
