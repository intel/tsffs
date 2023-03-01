use anyhow::{Context, Result, ensure};
use chrono::Local;
use confuse_simics_api::{SIM_VERSION, SIM_VERSION_COMPAT};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use object::{
    elf::FileHeader64,
    endian::LittleEndian,
    read::elf::{ElfFile, ElfFile64, ElfSymbol},
    Object, ObjectSection, ObjectSymbol,
};
use std::{
    cmp::min,
    fs::{read, OpenOptions},
    io::Write,
    num::Wrapping,
    path::{Path},
    time::{SystemTime, UNIX_EPOCH}, ascii::escape_default, iter::repeat
};
use whoami::username;

pub const MODULE_CAPABILITIES_SYMNAME: &str = "_module_capabilities_";
pub const MODULE_DATE_SYMNAME: &str = "_module_date";
pub const ELF_TEXT_SECTION_NAME: &str = ".text";
pub const ELF_DATA_SECTION_NAME: &str = ".data";
pub const MAX_SECTION_CSUM_SIZE: u64 = 256;
// Simics has a bug where it cannot handle a username longer than 20 characters in its signing
// check and may clobber the ELF is it sees a longer one. We won't allow that (20 chars + nul = 21)
pub const SIMICS_UNAME_LIMIT: usize = 20;
pub const SIMICS_SIGNATURE_LENGTH: usize = 44;

// Write the simics_constants.rs dynamically generated file for inclusion into the module
// We basically need to fake this file:
// /* module_id.c - automatically generated, do not edit */
//
// #include <simics/build-id.h>
// #include <simics/base/types.h>
// #include <simics/util/help-macros.h>
//
// #if defined(SIMICS_6_API)
// #define BUILD_API "6"
// #elif defined(SIMICS_5_API)
// #define BUILD_API "5"
// #elif defined(SIMICS_4_8_API)
// #define BUILD_API "4.8"
// #else
// #define BUILD_API "?"
// #endif
//
// #define EXTRA "                                           "
//
// EXPORTED const char _module_capabilities_[] =
//         "VER:" SYMBOL_TO_STRING(SIM_VERSION_COMPAT) ";"
//         "ABI:" SYMBOL_TO_STRING(SIM_VERSION) ";"
//         "API:" BUILD_API ";"
//         "BLD:" "0" ";"
//         "BLD_NS:__simics_project__;"
//          // date +'%s'
//         "BUILDDATE:" "1677199642" ";"
//         "MOD:" "afl-branch-tracer" ";"
//         "CLS:afl_branch_tracer" ";"
//         "HOSTTYPE:" "linux64" ";"
//         "THREADSAFE;"
//         EXTRA ";";
// // date +'%a %b %d %T %Y'
// EXPORTED const char _module_date[] = "Fri Feb 24 00:47:22 2023";
// EXPORTED void _simics_module_init(void);
// extern void sim_iface_wrap_init(void);
//
// void
// _simics_module_init(void)
// {
//
//         init_local();
// }
//
// The build process that produces it is:
//
// `/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/cctype.py --type gcc`
// `gcc -v`
// `gcc -dumpversion`
// `rm -rf linux64/obj/modules/`
// `ps -ax -o pid=,ppid=,pcpu=,pmem=,command=`
// `/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/project_setup.py --project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ --check-project-version`
// `/home/rhart/install/simics/simics-6.0.157/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/envcheck.py -c gcc linux64/.environment-check/all`
// `/home/rhart/install/simics/simics-6.0.157/bin/mini-python /home/rhart/install/simics/simics-6.0.157/scripts/build/module_id.py --c-module-id --output module_id.c --module-name afl-branch-tracer --classes afl_branch_tracer; --components  --host-type linux64 --thread-safe yes --user-init-local`
// `/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -E -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -M -MF module_id.d -MP -MT module_id.d -MT module_id.o -D HAVE_MODULE_DATE -D SIMICS_6_API module_id.c -mtune=generic -march=x86-64 -std=gnu99 -fvisibility=hidden -fasynchronous-unwind-tables -fstack-protector-strong -Wformat -Wformat-security -fstack-clash-protection -fcf-protection -dumpdir a- -dumpbase module_id.c -dumpbase-ext .c`
// `/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -E -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -M -MF afl-branch-tracer.d -MP -MT afl-branch-tracer.d -MT afl-branch-tracer.o -D HAVE_MODULE_DATE -D SIMICS_6_API /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer/afl-branch-tracer.c -mtune=generic -march=x86-64 -std=gnu99 -fvisibility=hidden -fasynchronous-unwind-tables -fstack-protector-strong -Wformat -Wformat-security -fstack-clash-protection -fcf-protection -dumpdir a- -dumpbase afl-branch-tracer.c -dumpbase-ext .c`
// `/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -I /home/rhart/install/simics/simics-6.0.157/linux64/bin/dml/include -imultiarch x86_64-linux-gnu -D HAVE_MODULE_DATE -D SIMICS_6_API -D _FORTIFY_SOURCE=2 /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer/afl-branch-tracer.c -quiet -dumpbase afl-branch-tracer.c -dumpbase-ext .c -mtune=generic -march=x86-64 -gdwarf-2 -O2 -Wall -Wwrite-strings -Wformat-security -std=gnu99 -fvisibility=hidden -fPIC -fasynchronous-unwind-tables -fstack-protector-strong -Wformat-security -fstack-clash-protection -fcf-protection -o /tmp/ccadVVa4.s`
// `/usr/lib/gcc/x86_64-linux-gnu/11/cc1 -quiet -I /home/rhart/install/simics/simics-6.0.157/src/include -I . -I /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/modules/afl-branch-tracer -imultiarch x86_64-linux-gnu -D HAVE_MODULE_DATE -D SIMICS_6_API -D _FORTIFY_SOURCE=2 module_id.c -quiet -dumpbase module_id.c -dumpbase-ext .c -mtune=generic -march=x86-64 -gdwarf-2 -O2 -Wall -Wwrite-strings -Wformat-security -std=gnu99 -fPIC -fasynchronous-unwind-tables -fstack-protector-strong -Wformat-security -fstack-clash-protection -fcf-protection -o /tmp/cckZqqtx.s`
// `/usr/lib/gcc/x86_64-linux-gnu/11/collect2 -plugin /usr/lib/gcc/x86_64-linux-gnu/11/liblto_plugin.so -plugin-opt=/usr/lib/gcc/x86_64-linux-gnu/11/lto-wrapper -plugin-opt=-fresolution=/tmp/ccvZgDwH.res -plugin-opt=-pass-through=-lgcc_s -plugin-opt=-pass-through=-lc -plugin-opt=-pass-through=-lgcc_s --build-id --eh-frame-hdr -m elf_x86_64 --hash-style=gnu --as-needed -shared -z relro -o /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so -z noexecstack -z relro -z now /usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu/crti.o /usr/lib/gcc/x86_64-linux-gnu/11/crtbeginS.o -L/home/rhart/install/simics/simics-6.0.157/linux64/bin -L/usr/lib/gcc/x86_64-linux-gnu/11 -L/usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu -L/usr/lib/gcc/x86_64-linux-gnu/11/../../../../lib -L/lib/x86_64-linux-gnu -L/lib/../lib -L/usr/lib/x86_64-linux-gnu -L/usr/lib/../lib -L/usr/lib/gcc/x86_64-linux-gnu/11/../../.. --version-script /home/rhart/install/simics/simics-6.0.157/config/project/exportmap.elf afl-branch-tracer.o module_id.o -lsimics-common -lvtutils -lstdc++ -lm -lgcc_s -lc -lgcc_s /usr/lib/gcc/x86_64-linux-gnu/11/crtendS.o /usr/lib/gcc/x86_64-linux-gnu/11/../../../x86_64-linux-gnu/crtn.o`
// `/home/rhart/install/simics/simics-6.0.157/bin/simics -project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ -batch-mode -quiet -no-copyright -no-module-cache -sign-module /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so`
// `/home/rhart/install/simics/simics-6.0.157/bin/../linux64/bin/simics-common -project /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ -batch-mode -quiet -no-copyright -no-module-cache -sign-module /tmp/test_minimal_simics_module_loads.m3mj3gjf5FVQ/linux64/lib/afl-branch-tracer.so`
//
// Basically:
// - Generate a C file
// - Compile against the C file
// - "Sign" the module (not sure what that means)

/// Generate a signature block for a simics module. This signature block contains API information
/// for SIMICS as well as date and time information. Critically, it includes space to place a
/// "signature" to make the module valid for use with SIMICS. Once a module has been built by
/// including this header, you must sign it before use with `sign_simics_module`.
pub fn generate_signature_header<P: AsRef<Path>, S: AsRef<str>>(
    crate_name: S,
    simics_home: P,
) -> Result<String> {
    // Probably this will be "6"
    let simics_latest = simics_latest(&simics_home)?;
    let simics_api = simics_latest.packages[&PackageNumber::Base]
        .version
        .split(".")
        .next()
        .context("No major version")?
        .trim();
    let crate_name = crate_name.as_ref().to_string();
    let class_name = crate_name.replace("-", "_");
    // This is the extra space where the signature will be inserted in the format:
    // "\x00CCCC\x00YYYY-MM-DD HH:MM;USER\x00"
    // Where "USER" may be up to 20 chars in length and is null-padded out to the EXTRA length
    // after signing but is initialized to ' '
    // const EXTRA: &str = "                                           ";
    let capabilities = vec![
        format!("VER:{}", SIM_VERSION_COMPAT),
        format!("ABI:{}", SIM_VERSION),
        format!("API:{}", simics_api),
        "BLD:0".to_string(),
        "BLD_NS:__simics_project__".to_string(),
        format!(
            "BUILDDATE:{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        ),
        format!("MOD:{}", crate_name),
        format!("CLS:{}", class_name),
        "HOSTTYPE:linux64".to_string(),
        "THREADSAFE".to_string(),
        repeat(" ").take(43).collect()
    ];
    let capabilities_string = capabilities.join(";") + ";" + "\x00";
    let capabilities_bytes  = capabilities_string.as_bytes().iter().map(|b| b.to_string()).collect::<Vec<_>>();
    let datetime_string = Local::now().format("%a %b %d %T %Y\x00").to_string();
    let datetime_bytes  = datetime_string.as_bytes().iter().map(|b| b.to_string()).collect::<Vec<_>>();
    let template = format!(
        r#"
    #[no_mangle]
    /// This bytestring documents the capabilities of the module in the format output by
    /// the module_id.py script distributed by simics. Specifically, the script is
    /// called like:
    /// module_id.py --c-module-id --output module_id.c --module-name NAME --classes "CLASS1;CLASS2;" \
    ///     --components "" --host-type linux64 --thread-safe yes --user-init-local
    pub static _module_capabilities_: [u8; {}] = [{}];

    #[no_mangle]
    /// This bytestring records the date in +'%a %b %d %T %Y' format
    pub static _module_date: [u8; {}] = [{}];

    #[no_mangle]
    pub extern "C" fn _simics_module_init() {{
        init_local();
    }}

    const CLASS_NAME: &str = "{}";
    "#,
        capabilities_bytes.len(), capabilities_bytes.join(", "), datetime_bytes.len(), datetime_bytes.join(", "), class_name
    );

    Ok(template)
}

pub fn parse_module<'data>(
    module: &'data [u8],
) -> Result<ElfFile<'data, FileHeader64<LittleEndian>>> {
    Ok(ElfFile64::parse(module)?)
}

pub fn get_mod_capabilities<'data, 'file>(
    elf: &'file ElfFile<'data, FileHeader64<LittleEndian>>,
) -> Result<ElfSymbol<'data, 'file, FileHeader64<LittleEndian>>> {
    Ok(elf
        .symbols()
        .find(|s| s.name() == Ok(MODULE_CAPABILITIES_SYMNAME))
        .context("No symbol _module_capabilities_ found")?)
}

pub fn sign_simics_module<P: AsRef<Path>>(module: P) -> Result<()> {
    let username = username();
    Ok(sign_simics_module_as(username, module)?)
}

pub fn sign_simics_module_as<P: AsRef<Path>, S: AsRef<str>>(uname: S, module: P) -> Result<()> {
    let data = read(&module)?;
    let idata = &data[..];
    let signed_module_data = sign_simics_module_data(uname, idata)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&module)?;

    file.write_all(&signed_module_data)?;

    Ok(())
}

pub fn calculate_module_checksum(elf: &ElfFile<FileHeader64<LittleEndian>>) -> Result<Wrapping<u32>> {
    let text_section = elf
        .section_by_name(ELF_TEXT_SECTION_NAME)
        .context("No text section found.")?;
    let data_section = elf
        .section_by_name(ELF_DATA_SECTION_NAME)
        .context("No text section found.")?;

    // Checksum initialized to Wrapping<1u32>
    let mut text_section_data = text_section.data()?.to_vec();
    text_section_data.truncate(min(text_section.size(), MAX_SECTION_CSUM_SIZE) as usize);
    let mut data_section_data = data_section.data()?.to_vec();
    data_section_data.truncate(min(data_section.size(), MAX_SECTION_CSUM_SIZE) as usize);

    let csum: Wrapping<u32> = 
    // Checksum starts with 1 instead of 0 because it is multiplicative
        Wrapping(1u32)
        * (
            // The size of the text section
            Wrapping(text_section.size() as u32)
            * 
            // The sum of the first 256 bytes or entirety of the text section, whichever is smaller
            text_section_data
                .iter()
                .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32)))
        * (
            // The size of the data section
            Wrapping(data_section.size() as u32)
            * 
            // The sum of the first 256 bytes or entirety of the data section, whichever is smaller
            data_section_data
                .iter()
                .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32)))
        // OR with 1 to set the lsb to prevent the csum from ever equaling '    ' (0x20202020)
        | Wrapping(1u32);

    Ok(csum)

}

pub fn get_mod_capabilities_data<'data, 'file>(elf: &'file ElfFile<'data, FileHeader64<LittleEndian>>) -> Result<Vec<u8>> {
    let mod_capabilities = get_mod_capabilities(&elf)?;
    let data = elf.data();
    let mod_capabilities_data = &data[mod_capabilities.address() as usize..mod_capabilities.address() as usize + mod_capabilities.size() as usize];
    Ok(mod_capabilities_data.to_vec())
}

pub fn generate_signature_data<'data, 'file, S: AsRef<str>>(uname: S, elf: &'file ElfFile<'data, FileHeader64<LittleEndian>>) -> Result<Vec<u8>> {
    let checksum = calculate_module_checksum(&elf)?;
    let csum_bytes: [u8; 4] = checksum.0.to_le_bytes();
    let mod_capabilities_data = get_mod_capabilities_data(&elf)?;
    let split_seq = b"; ";
    let sign_pos = mod_capabilities_data.windows(split_seq.len()).position(|w| w == split_seq).context(format!(
        "Sequence '{:?}' not found in byte string '{}'.",
        split_seq,
        String::from_utf8(mod_capabilities_data.iter().flat_map(|b| escape_default(*b)).collect())?
    ))? + split_seq.len();
    // TODO: This may not actually need to be getlogin() but that's what simics uses, so we'll keep it consistent
    let mut uname = uname.as_ref().to_string();
    uname.truncate(20);
    let datetime_string = Local::now().format("%Y-%M-%d %H:%M").to_string();
    let mut signature_data = Vec::new();
    signature_data.push(0u8);
    signature_data.extend(csum_bytes);
    signature_data.push(0u8);
    signature_data.extend(datetime_string.as_bytes());
    signature_data.push(b';');
    signature_data.extend(uname.as_bytes());
    signature_data.resize(SIMICS_SIGNATURE_LENGTH, 0u8);
    let mut signature = mod_capabilities_data[..sign_pos].to_vec();
    signature.extend(signature_data);
    ensure!(signature != mod_capabilities_data, "Signature is the same as original signature.");
    Ok(signature)
}

pub fn sign_simics_module_data<S: AsRef<str>>(uname: S, module: &[u8]) -> Result<Vec<u8>> {
    let elf = parse_module(module)?;
    let mod_capabilities = get_mod_capabilities(&elf)?;
    let signature_data = generate_signature_data(uname, &elf)?;
    let data = elf.data().to_vec();

    let pre_sig = data[..mod_capabilities.address() as usize].to_vec();
    let post_sig = data[mod_capabilities.address() as usize + mod_capabilities.size() as usize..].to_vec();
    let signed = pre_sig.iter().chain(signature_data.iter()).chain(post_sig.iter()).cloned().collect::<Vec<_>>();

    ensure!(module.len() == signed.len(), "Signed module length differs from input module.");
    ensure!(module != &signed, "Signed module is the same as the input module.");

    Ok(signed)
}