use anyhow::{Context, Result, ensure};
use cargo_metadata::MetadataCommand;
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
    fs::{read, OpenOptions, create_dir_all},
    io::Write,
    num::Wrapping,
    path::{Path, PathBuf},
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

#[derive(Hash, PartialEq, Eq)]
pub struct SimicsModule {
    /// The path the module is at when the instance of `SimicsModule` is created
    /// and before it is signed and copied to its new home
    pub original_path: PathBuf,
    pub path: PathBuf,
    pub project_base_path: PathBuf,
    pub name: String,
    pub class_name: String,
}

impl SimicsModule {
    pub fn try_new<S: AsRef<str>, P: AsRef<Path>>(crate_name: S, project_base_path: P, module_path: P) -> Result<Self> {
        let original_path = module_path.as_ref().to_path_buf();
        let project_base_path = project_base_path.as_ref().to_path_buf();
        let username = username();
        let name = crate_name.as_ref().to_string();
        let class_name = name.replace("-", "_");
        let module_dir = project_base_path.join("linux64").join("lib");
        let path = module_dir.join(original_path.file_name().context("No filename in module path")?);

        let module = SimicsModule {
            original_path,
            path,
            project_base_path,
            name,
            class_name
        };

        create_dir_all(&module_dir)?;
        sign_move_simics_module_as(username,  &module.original_path, &module.path)?;

        Ok(module)
    }

}

pub fn find_module<S: AsRef<str>>(crate_name: S) -> Result<PathBuf> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let ws_root = metadata.workspace_root;
    let workspace_metadata = MetadataCommand::new()
        .no_deps()
        .manifest_path(ws_root.join("Cargo.toml"))
        .exec()?;

    let target = workspace_metadata.packages
        .iter()
        .filter(|p| p.name == crate_name.as_ref() && p.targets.iter().filter(|t| t.is_lib()).next().is_some())
        .filter_map(|p| p.targets.iter().filter(|t| t.is_lib()).next())
        .take(1)
        .next()
        .context("No package with given crate name.")?;

    #[cfg(debug_assertions)]
    let target_subdir = "debug";
    #[cfg(not(debug_assertions))]
    let target_subdir = "release";

    let lib_path = workspace_metadata.target_directory.join(target_subdir).join(format!("lib{}.so", target.name.replace("-", "_")));

    Ok(lib_path.into())
}

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

    pub const CRATE_NAME: &str = "{}";
    pub const CLASS_NAME: &str = "{}";
    "#,
        capabilities_bytes.len(), capabilities_bytes.join(", "), datetime_bytes.len(), datetime_bytes.join(", "), crate_name, class_name
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

pub fn sign_move_simics_module_as<P: AsRef<Path>, S: AsRef<str>>(uname: S, module: P, dest: P) -> Result<()> {
    let data = read(&module)?;
    let idata = &data[..];
    let signed_module_data = sign_simics_module_data(uname, idata)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dest)?;

    file.write_all(&signed_module_data)?;

    Ok(())
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