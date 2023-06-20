use anyhow::{anyhow, Result};
use artifact_dependency::{ArtifactDependencyBuilder, CrateType};
use clap::Parser;
use libafl::bolts::core_affinity::Cores;
use simics::{
    api::sys::SIMICS_VERSION,
    module::ModuleBuilder,
    project::{Project, ProjectBuilder, ProjectPathBuilder},
};
use simics_fuzz::{
    args::Args,
    fuzzer::SimicsFuzzerBuilder,
    modules::confuse::{CONFUSE_MODULE_CRATE_NAME, CONFUSE_WORKSPACE_PATH},
};
use std::path::PathBuf;
use tracing::trace;
use tracing_subscriber::fmt::{self, format};

pub fn main() -> Result<()> {
    let args = Args::parse();

    fmt::fmt()
        .pretty()
        .with_max_level(args.log_level)
        .try_init()
        .map_err(|e| anyhow!("Couldn't initialize tracing subscriber: {}", e))?;

    trace!("Setting up project with args: {:?}", args);

    let mut builder: ProjectBuilder = if let Some(project_path) = args.project {
        if let Ok(project) = Project::try_from(project_path.clone()) {
            project.into()
        } else {
            // TODO: Merge with else branch, they are practically the same code.
            let mut builder = ProjectBuilder::default();

            builder.path(
                ProjectPathBuilder::default()
                    .path(project_path.clone())
                    .temporary(args.no_keep_temp_projects)
                    .build()?,
            );

            args.package.into_iter().for_each(|p| {
                builder.package(p.package);
            });
            args.module.into_iter().for_each(|m| {
                builder.module(m.module);
            });
            args.directory.into_iter().for_each(|d| {
                builder.directory((d.src, d.dst));
            });
            args.file.into_iter().for_each(|f| {
                builder.file((f.src, f.dst));
            });
            args.path_symlink.into_iter().for_each(|s| {
                builder.path_symlink((s.src, s.dst));
            });

            builder
        }
    } else {
        if let Ok(project) = Project::try_from(PathBuf::from(".")) {
            project.into()
        } else {
            let mut builder = ProjectBuilder::default();

            args.package.into_iter().for_each(|p| {
                builder.package(p.package);
            });
            args.module.into_iter().for_each(|m| {
                builder.module(m.module);
            });
            args.directory.into_iter().for_each(|d| {
                builder.directory((d.src, d.dst));
            });
            args.file.into_iter().for_each(|f| {
                builder.file((f.src, f.dst));
            });
            args.path_symlink.into_iter().for_each(|s| {
                builder.path_symlink((s.src, s.dst));
            });

            builder
        }
    };

    let project = builder
        .module(
            ModuleBuilder::default()
                .artifact(
                    ArtifactDependencyBuilder::default()
                        .crate_name(CONFUSE_MODULE_CRATE_NAME)
                        .workspace_root(PathBuf::from(CONFUSE_WORKSPACE_PATH))
                        .build_missing(true)
                        .artifact_type(CrateType::CDynamicLibrary)
                        .feature(SIMICS_VERSION)
                        .build()?
                        .build()?,
                )
                .build()?,
        )
        .build()?
        .setup()?;

    SimicsFuzzerBuilder::default()
        .project(project)
        .input(args.input)
        .corpus(args.corpus)
        .solutions(args.solutions)
        .tui(args.tui)
        .grimoire(args.grimoire)
        .cores(Cores::from((0..args.cores).collect::<Vec<_>>()))
        .command(args.command)
        .build()?
        .launch()?;

    Ok(())
}
