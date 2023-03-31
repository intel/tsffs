//! Utilities for defining an app YAML with rust macros

use indoc::formatdoc;

const YAML_INDENT: &str = "  ";

/// Params to a simics app can be one of four types: int, file, bool, or str. They don't
/// necessarily have a "default" which is the value stored in this enum
pub enum SimicsAppParamType {
    Int(Option<i64>),
    File(Option<String>),
    Bool(Option<bool>),
    Str(Option<String>),
}

/// Parameter to a simics app, these always have a type, may have a default (if the default is
/// not provided, it must be set by the app's script), and they may set the boolean `output`.
pub struct SimicsAppParam {
    pub default: SimicsAppParamType,
    pub output: Option<bool>,
}

impl ToString for SimicsAppParam {
    fn to_string(&self) -> String {
        let mut pstr = vec![format!(
            "type: {}",
            match &self.default {
                SimicsAppParamType::Int(_) => "int",
                SimicsAppParamType::File(_) => "file",
                SimicsAppParamType::Str(_) => "str",
                SimicsAppParamType::Bool(_) => "bool",
            }
        )];

        match &self.default {
            SimicsAppParamType::Int(Some(v)) => {
                pstr.push(format!("default: {}", v));
            }
            SimicsAppParamType::File(Some(v)) => pstr.push(format!(r#"default: "{}""#, v)),
            SimicsAppParamType::Str(Some(v)) => pstr.push(format!(r#"default: "{}""#, v)),
            // Yet more inconsistency with YAML spec
            SimicsAppParamType::Bool(Some(v)) => pstr.push(format!(
                "default: {}",
                match v {
                    true => "TRUE",
                    false => "FALSE",
                }
            )),
            _ => {}
        };

        if let Some(output) = self.output {
            pstr.push(format!("output: {}", output));
        }

        pstr.iter()
            .map(|e| YAML_INDENT.to_string() + e)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl SimicsAppParam {
    pub fn new(typ: SimicsAppParamType) -> Self {
        Self {
            default: typ,
            output: None,
        }
    }

    pub fn set_output(&mut self, value: bool) {
        self.output = Some(value);
    }

    pub fn set_default(&mut self, value: SimicsAppParamType) {
        self.default = value;
    }
}

pub struct SimicsApp {
    pub description: String,
    pub params: Vec<(String, SimicsAppParam)>,
    pub script: String,
}

impl SimicsApp {
    pub fn new<S: AsRef<str>>(description: S, script: S) -> Self {
        Self {
            description: description.as_ref().to_string(),
            params: Vec::new(),
            script: script.as_ref().to_string(),
        }
    }

    pub fn param<S: AsRef<str>>(&mut self, key: S, param: SimicsAppParam) -> &mut Self {
        self.params.push((key.as_ref().to_string(), param));
        self
    }

    pub fn params_string(&self) -> String {
        self.params
            .iter()
            .map(|(k, p)| {
                format!("{}:\n{}", k, p.to_string())
                    .lines()
                    .map(|l| YAML_INDENT.to_string() + l)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn add_param<S: AsRef<str>>(&mut self, key: S, param: SimicsAppParam) {
        self.params.push((key.as_ref().to_string(), param));
    }
}

impl ToString for SimicsApp {
    fn to_string(&self) -> String {
        formatdoc! {r#"
            %YAML 1.2
            ---
            description: {}
            params:
            {}
            script: "{}"
            ...
            "#, 
            self.description,
            self.params_string(),
            self.script
        }
    }
}

#[macro_export]
macro_rules! int_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_default(SimicsAppParamType::Int(Some($dval)));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_default(SimicsAppParamType::Int(Some($dval)));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! str_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_default(SimicsAppParamType::Str(Some($dval.into())));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_default(SimicsAppParamType::Str(Some($dval.into())));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! file_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_default(SimicsAppParamType::File(Some($dval.into())));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_default(SimicsAppParamType::File(Some($dval.into())));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! bool_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_default(SimicsAppParamType::Bool(Some($dval)));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_default(SimicsAppParamType::Bool(Some($dval)));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! simics_app {
    ($description:expr, $script:expr, $($param:expr),* $(,)?) => {
        {
            let mut app = SimicsApp::new($description, $script);
            $(
                app.add_param($param.0, $param.1);
            )*
            app
        }
    }
}

#[macro_export]
/// Create a path relative to the simics project directory.
///
/// # Examples
///
/// ```
/// const SCRIPT_PATH: &str = "scripts/app.py";
/// let app = SimicsApp::new("An app", &simics_path!(SCRIPT_PATH));
/// assert_eq!(app.script, "%simics%/scripts/app.py");
/// ```
macro_rules! simics_path {
    ($path:expr) => {
        format!("%simics%/{}", $path)
    };
}
