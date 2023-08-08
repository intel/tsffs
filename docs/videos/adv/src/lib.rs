// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]

use anyhow::{anyhow, bail, Result};
use derive_builder::Builder;
use indicatif::ProgressBar;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};
use strfmt::Format;
use toml::from_str;

#[derive(Debug, Clone)]
pub enum StartAs {
    Normal,
    Fullscreen,
    Minimized,
    Maximized,
}

impl ToString for StartAs {
    fn to_string(&self) -> String {
        match self {
            StartAs::Normal => "normal",
            StartAs::Fullscreen => "fullscreen",
            StartAs::Minimized => "minimized",
            StartAs::Maximized => "maximized",
        }
        .to_string()
    }
}

#[derive(Builder)]
pub struct KittyConfig {
    #[builder(setter(into, strip_option), default)]
    class: Option<String>,
    #[builder(setter(into, strip_option), default)]
    name: Option<String>,
    #[builder(setter(into, strip_option), default)]
    title: Option<String>,
    #[builder(setter(into, strip_option), default)]
    config: Option<PathBuf>,
    #[builder(setter(custom))]
    overrides: Vec<(String, String)>,
    #[builder(setter(into, strip_option), default)]
    directory: Option<PathBuf>,
    #[builder(default)]
    detach: bool,
    #[builder(setter(into, strip_option), default)]
    session: Option<PathBuf>,
    #[builder(default)]
    hold: bool,
    #[builder(default)]
    single_instance: bool,
    #[builder(setter(into, strip_option), default)]
    instance_group: Option<String>,
    #[builder(default)]
    wait_for_single_instance_window_close: bool,
    #[builder(setter(into, strip_option), default)]
    listen_on: Option<String>,
    #[builder(setter(into, strip_option), default = "StartAs::Normal")]
    start_as: StartAs,
}

impl KittyConfigBuilder {
    pub fn overrides<I, S>(&mut self, value: I) -> &mut Self
    where
        for<'a> &'a I: IntoIterator<Item = &'a (S, S)>,
        S: AsRef<str>,
    {
        self.overrides = Some(
            value
                .into_iter()
                .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
                .collect(),
        );
        self
    }
}

impl KittyConfig {
    fn to_args(&self) -> Result<Vec<String>> {
        let mut args = Vec::new();

        if let Some(c) = self.class.as_ref() {
            args.push("--class".to_string());
            args.push(c.to_string())
        }

        if let Some(n) = self.name.as_ref() {
            args.push("--name".to_string());
            args.push(n.to_string())
        }

        if let Some(t) = self.title.as_ref() {
            args.push("--title".to_string());
            args.push(t.to_string())
        }

        if let Some(c) = self.config.as_ref() {
            args.push("--config".to_string());
            args.push(c.to_string_lossy().to_string());
        }

        for (k, v) in &self.overrides {
            args.push("--override".to_string());
            args.push(format!("{}={}", k, v));
        }

        if let Some(d) = self.directory.as_ref() {
            args.push("--working-directory".to_string());
            args.push(d.to_string_lossy().to_string());
        }

        if self.detach {
            args.push("--detach".to_string());
        }

        if let Some(s) = self.session.as_ref() {
            args.push("--session".to_string());
            args.push(s.to_string_lossy().to_string());
        }

        if self.hold {
            args.push("--hold".to_string());
        }

        if self.single_instance {
            args.push("--single-instance".to_string());
        }

        if let Some(ig) = self.instance_group.as_ref() {
            args.push("--instance-group".to_string());
            args.push(ig.to_string());
        }

        if self.wait_for_single_instance_window_close {
            args.push("--wait-for-single-instance-window-close".to_string());
        }

        if let Some(l) = self.listen_on.as_ref() {
            args.push("--listen-on".to_string());
            args.push(l.to_string());
        }

        match self.start_as {
            StartAs::Normal => {}
            StartAs::Fullscreen => {
                args.push("--start-as".to_string());
                args.push(StartAs::Fullscreen.to_string());
            }
            StartAs::Minimized => {
                args.push("--start-as".to_string());
                args.push(StartAs::Minimized.to_string());
            }
            StartAs::Maximized => {
                args.push("--start-as".to_string());
                args.push(StartAs::Maximized.to_string());
            }
        }

        Ok(args)
    }
}

pub struct KittyProcess {
    config: KittyConfig,
    kitty: Child,
}

impl KittyProcess {
    pub fn try_new(config: KittyConfig, secrets: &Secrets) -> Result<Self> {
        let kitty = Command::new("kitty")
            .args(config.to_args()?)
            .envs(secrets.as_ref())
            .spawn()?;

        Ok(Self { config, kitty })
    }

    // Send text to the kitty process. Text can contain escape characters that
    // will be unescaped by the receiving process.
    pub fn send_text<S>(&self, text: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        let selector = if let Some(l) = self.config.listen_on.as_ref() {
            vec!["--to".to_string(), l.to_string()]
        } else if let Some(t) = self.config.title.as_ref() {
            vec!["--match".to_string(), format!("title:{}", t)]
        } else {
            bail!("No way to select window");
        };

        Command::new("kitty")
            .arg("@")
            .arg("send-text")
            .args(selector)
            .arg(text.as_ref())
            .spawn()?
            .wait()
            .map_err(|e| anyhow!("Failed to wait for send-text: {}", e))
            .and_then(|s| {
                if s.success() {
                    Ok(s)
                } else {
                    Err(anyhow!("Failed to send text"))
                }
            })?;

        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        self.kitty
            .kill()
            .map_err(|e| anyhow!("Couldn't kill kitty: {}", e))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommandConfig {
    /// Command that will be executed. Doesn't actually have to be a command in the usual sense,
    /// You can have a command that is:
    ///
    /// `command = "ls -lah\n"`
    ///
    /// But you can also have a command that is just:
    ///
    /// `command = "Y\n"`
    ///
    /// To accept a prompt or some such thing.
    pub command: String,
    /// Delay before the prompt, if there is one. If there is a `speech_pre`, the pre speech will
    /// begin at a time offset into this delay equal to its length, such that it finishes just
    /// before the command execution.
    pub delay_pre: Option<f32>,
    /// Delay after command. This is the most useful delay, and it runs concurrently with
    /// `speech_post`, so you can have narration after a command for a command that takes a
    /// set amount of time (you don't need to rely on the speech being a parciular length
    /// of time). Note that if speech takes longer than this delay, it will finish before
    /// moving on to the next command.
    pub delay_post: Option<f32>,
    /// Whether to prompt before the execution. This is useful for waits that are not able to
    /// reliably be timed and require a visual to confirm and move on.
    pub prompt: Option<bool>,
    /// Words per minute to type the command at. If not specified, the command will be typed
    /// instantaneously.
    pub wpm: Option<u16>,
    /// Script to print out before the command is executed and before the delay starts.
    pub script_pre: Option<String>,
    /// Script to print out after the command is executed and before the delay starts.
    pub script_post: Option<String>,
    /// Whether WPM settings should be bypassed to run immediately
    pub immediate: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KittyOptions {
    pub font_size: Option<f32>,
    pub allow_remote_control: Option<String>,
    pub remote_control_password: Option<String>,
}

impl Default for KittyOptions {
    fn default() -> Self {
        Self {
            font_size: None,
            allow_remote_control: Some("yes".to_string()),
            remote_control_password: None,
        }
    }
}

impl KittyOptions {
    pub fn to_overrides(&self) -> Vec<(String, String)> {
        let mut overrides = Vec::new();

        if let Some(font_size) = self.font_size.as_ref() {
            overrides.push(("font_size".to_string(), font_size.to_string()));
        }

        if let Some(allow_remote_control) = self.allow_remote_control.as_ref() {
            overrides.push((
                "allow_remote_control".to_string(),
                allow_remote_control.to_string(),
            ));
        }

        if let Some(remote_control_password) = self.remote_control_password.as_ref() {
            overrides.push((
                "remote_control_password".to_string(),
                remote_control_password.to_string(),
            ))
        }

        overrides
    }
}

fn pretty_print_script<S>(script: S, pre: bool, command: S)
where
    S: AsRef<str>,
{
    let longest_len = script
        .as_ref()
        .lines()
        .map(|l| l.len())
        .max()
        .unwrap_or_default();

    println!("{}", "-".repeat(longest_len));
    println!(
        "{}Script for: ```{}````",
        if pre { "Pre-" } else { "" },
        command.as_ref().trim()
    );
    println!("{}", "-".repeat(longest_len));
    println!("{}", script.as_ref());
    println!("{}", "-".repeat(longest_len));
}

fn sleep_bar(mut seconds: f32) -> Result<()> {
    let bar = ProgressBar::new(seconds as u64);
    while seconds > 0.0 {
        let sleep_time = if seconds > 1.0 { 1.0 } else { seconds };
        sleep(Duration::from_secs_f32(sleep_time));
        seconds -= sleep_time;
        bar.inc(1);
    }
    bar.finish_and_clear();

    Ok(())
}

fn unescape<S>(string: S) -> Result<String>
where
    S: AsRef<str>,
{
    let mut chars = string.as_ref().chars();
    let mut output = String::new();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\\') => {
                    if let Some(c) = chars.next() {
                        output.push('\\');
                        output.push(c);
                    } else {
                        output.push('\\');
                    }
                }
                Some(n) => match n {
                    'e' => output.push('\x1b'),
                    'n' => output.push('\n'),
                    'b' => output.push('\x08'),
                    '"' => output.push('\"'),
                    _ => {}
                },
                None => {}
            },
            _ => {
                output.push(c);
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod unescape_tests {
    use crate::unescape;

    #[test]
    fn test_unescape() {
        assert_eq!(unescape(r#"\\n"#).unwrap(), r#"\n"#);
        assert_eq!(unescape(r#"\n"#).unwrap(), "\n");
        assert_eq!(unescape(r#"\e"#).unwrap(), r#"\e"#);
    }
}

impl CommandConfig {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn exec(&self, kitty: &KittyProcess, secrets: &Secrets) -> Result<()> {
        let pre_delay = self.delay_pre.unwrap_or(0.0);

        if let Some(script_pre) = self.script_pre.as_ref() {
            pretty_print_script(script_pre, true, &self.command);
        }

        sleep_bar(pre_delay)?;

        if self.prompt.unwrap_or(false) {
            print!("Press Enter to continue: ");
            stdout().flush()?;
            stdin()
                .lines()
                .take(1)
                .next()
                .ok_or_else(|| anyhow!("Failed to get a line"))??;
        }

        let command = unescape(
            self.command
                .clone()
                .format(secrets.as_ref())
                .unwrap_or(self.command.clone()),
        )?;

        if let Some(wpm) = self.wpm {
            if wpm == 0 || self.immediate.unwrap_or_default() {
                kitty.send_text(command)?;
            } else {
                let seconds_per_char = 60.0 / (wpm as f32 * 5.0);
                for character in command.chars() {
                    let jitter: f32 = thread_rng().gen_range(0.01..0.04);
                    let wait = seconds_per_char + jitter;
                    sleep(Duration::from_secs_f32(wait));
                    kitty.send_text(character.to_string())?;
                }
            }
        } else {
            kitty.send_text(command)?;
        }

        if let Some(script_pre) = self.script_post.as_ref() {
            pretty_print_script(script_pre, false, &self.command);
        }

        let post_delay = self.delay_post.unwrap_or(0.0);

        sleep_bar(post_delay)?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub kitty: Option<KittyOptions>,
    pub secrets: Option<Vec<String>>,
    pub secrets_file: Option<PathBuf>,
    pub wpm: Option<u16>,
    pub keep_open: Option<bool>,
    pub command: Vec<CommandConfig>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Secrets {
    secrets: HashMap<String, String>,
}

impl Secrets {
    pub fn new<P>(secrets_file: Option<P>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        if let Some(secrets_file) = secrets_file.as_ref() {
            from_str(&read_to_string(secrets_file.as_ref())?).map_err(|e| {
                anyhow!(
                    "Couldn't read secrets file {}: {}",
                    secrets_file.as_ref().display(),
                    e
                )
            })
        } else {
            Ok(Self::default())
        }
    }

    pub fn get<S>(&self, secret: S) -> Result<String>
    where
        S: AsRef<str>,
    {
        self.secrets
            .get(secret.as_ref())
            .cloned()
            .ok_or_else(|| anyhow!("No secret found named {}", secret.as_ref()))
    }

    pub fn add<S>(&mut self, name: S, secret: S)
    where
        S: AsRef<str>,
    {
        self.secrets
            .insert(name.as_ref().to_string(), secret.as_ref().to_string());
    }
}

impl AsRef<HashMap<String, String>> for Secrets {
    fn as_ref(&self) -> &HashMap<String, String> {
        &self.secrets
    }
}
