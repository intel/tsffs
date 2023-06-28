use anyhow::Result;
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
use std::{io::stderr, path::PathBuf};
use tracing::{trace, Level};
use tracing_subscriber::{filter::filter_fn, fmt, prelude::*, registry, Layer};

pub fn main() -> Result<()> {
    let args = Args::parse();

    registry()
        .with(
            fmt::layer()
                .pretty()
                .with_writer(stderr)
                .with_filter(args.log_level)
                .with_filter(filter_fn(|metadata| {
                    // LLMP absolutely spams the log when tracing
                    !(metadata.target() == "libafl::bolts::llmp"
                        && matches!(metadata.level(), &Level::TRACE))
                })),
        )
        .init();

    trace!("Setting up project with args: {:?}", args);

    let mut builder: ProjectBuilder = if let Some(project_path) = args.project {
        if let Ok(project) = Project::try_from(project_path.clone()) {
            project.into()
        } else {
            // TODO: Merge with else branch, they are practically the same code.
            let mut builder = ProjectBuilder::default();

            builder.path(
                ProjectPathBuilder::default()
                    .path(project_path)
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
    } else if let Ok(project) = Project::try_from(PathBuf::from(".")) {
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
                        .target_name("simics-fuzz")
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
        ._grimoire(args.grimoire)
        .cores(Cores::from((0..args.cores).collect::<Vec<_>>()))
        .command(args.command)
        .timeout(args.timeout)
        .executor_timeout(args.executor_timeout)
        .log_level(args.log_level)
        .trace_mode(args.trace_mode)
        .build()?
        .launch()?;

    Ok(())
}
