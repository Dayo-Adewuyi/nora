use std::path::PathBuf;

use chew_corpus::{CorpusError, PipelineRunner, RunReport};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "chew-corpus-pipeline")]
#[command(about = "Offline extraction and validation for the CHPRBN corpus")]
struct Cli {
    #[arg(long, default_value = ".", global = true)]
    repo_root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Extract layout-preserving source artifacts.
    Extract(ConfigArgs),
    /// Compare text and illustrated editions.
    Compare(ConfigArgs),
    /// Extract, compare, and publish reports.
    Run(ConfigArgs),
    /// Verify local sources and the PDF toolchain.
    Verify(ConfigArgs),
}

#[derive(Debug, clap::Args)]
struct ConfigArgs {
    #[arg(long, default_value = "corpus/pipeline.json")]
    config: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let result = execute(cli);
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn execute(cli: Cli) -> Result<(), CorpusError> {
    let runner = PipelineRunner::new(cli.repo_root)?;
    match cli.command {
        Command::Verify(args) => {
            let report = runner.verify(&args.config)?;
            println!(
                "verified {} sources with Poppler {}",
                report.sources.len(),
                report.poppler_version
            );
            Ok(())
        }
        Command::Extract(args) | Command::Compare(args) | Command::Run(args) => {
            finish_run(runner.run(&args.config)?)
        }
    }
}

fn finish_run(report: RunReport) -> Result<(), CorpusError> {
    println!(
        "published {} sources and {} cadre reports; {} blocking issues; semantic digest {}",
        report.sources.len(),
        report.comparisons.len(),
        report.blocking_issues,
        report.semantic_artifact_sha256
    );
    if report.blocking_issues > 0 {
        return Err(CorpusError::InvalidExtraction(format!(
            "report contains {} blocking validation issues",
            report.blocking_issues
        )));
    }
    Ok(())
}
