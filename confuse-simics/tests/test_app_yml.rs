use anyhow::{bail, Result};
use confuse_simics::{SimicsApp, SimicsAppParamType};
use serde_yaml::from_reader;

const TEST_APP_YML: &[u8] = include_bytes!("rsrc/qsp-x86-uefi-app.yml");

#[test]
fn test_parse_app_yml() -> Result<()> {
    let app: SimicsApp = from_reader(TEST_APP_YML)?;
    assert_eq!(
        app.description, "QSP with UEFI App (Fuzzing)",
        "Incorrect description."
    );
    match app.params.get("disk0_size") {
        Some(p) => match p.param {
            SimicsAppParamType::Int(pt) => {
                assert!(pt.is_some() && pt.unwrap() == 209715200);
            }
            _ => bail!("Incorrect param type"),
        },
        None => bail!("Failed to get disk0_size param"),
    }

    Ok(())
}
