//! NODS CLI — manage terminal-workspace configs and sessions from the terminal.
//!
//! Mirrors the command surface of the Tillio CLI (login / status / doctor /
//! metrics / completion + domain verbs) but is nods-specific and Rust-native.

mod client;
mod commands;
mod config;
mod exit_codes;
mod output;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use output::Format;

#[derive(Parser)]
#[command(
    name = "nods",
    version,
    about = "NODS CLI — terminal-workspace orchestrator",
    long_about = "Manage NODS terminal-workspace configs and sessions from your terminal.\n\
                  Talks to the NODS API (default https://api.nods.dev; override with NODS_API_URL or --api-url)."
)]
struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Table, global = true)]
    output: OutputFormat,

    /// Override the API base URL (else $NODS_API_URL or config)
    #[arg(long, global = true)]
    api_url: Option<String>,

    /// Show informational messages on stderr
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

impl From<OutputFormat> for Format {
    fn from(o: OutputFormat) -> Self {
        match o {
            OutputFormat::Table => Format::Table,
            OutputFormat::Json => Format::Json,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Store your NODS API key (from the product app: Account > Developer)
    Login {
        /// Provide the key non-interactively (else you are prompted)
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Show your account and active-session summary
    Status,
    /// Diagnose CLI setup and API connectivity
    Doctor,
    /// Show usage / performance analytics
    Metrics {
        /// Which analytics view to fetch
        #[arg(value_enum, default_value_t = MetricKind::Usage)]
        kind: MetricKind,
    },
    /// Generate a shell-completion script
    Completion {
        /// Target shell
        #[arg(value_enum, default_value_t = Shell::Zsh)]
        shell: Shell,
    },
    /// Manage workspace configurations
    #[command(subcommand)]
    Configs(ConfigsCmd),
    /// Manage terminal sessions
    #[command(subcommand)]
    Sessions(SessionsCmd),
    /// Browse the public template library
    #[command(subcommand)]
    Templates(TemplatesCmd),
    /// List recent account events
    Events {
        /// Filter by event type
        #[arg(long)]
        r#type: Option<String>,
        /// Max rows
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Show live system resource usage for your sessions
    Resources,
}

#[derive(Copy, Clone, ValueEnum)]
enum MetricKind {
    Usage,
    Performance,
    Backends,
    Daily,
    Errors,
}

#[derive(Subcommand)]
enum ConfigsCmd {
    /// List your workspace configs
    List,
    /// Show one config by id
    Show { id: String },
    /// Validate a config by id
    Validate { id: String },
    /// Delete a config by id
    Delete { id: String },
}

#[derive(Subcommand)]
enum SessionsCmd {
    /// List sessions (optionally filter by status)
    List {
        #[arg(long, value_parser = ["active", "stopped"])]
        status: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Show one session by id
    Show { id: String },
    /// Launch a session from a config id
    Launch { config_id: String },
    /// Kill (stop) a session by id
    Kill { id: String },
}

#[derive(Subcommand)]
enum TemplatesCmd {
    /// List public templates
    List,
    /// Show one template by id
    Show { id: String },
}

fn main() {
    let cli = Cli::parse();
    let fmt: Format = cli.output.into();
    let api_url = cli.api_url.as_deref();

    // Warn (stderr) about a malformed config file before doing anything.
    let cfg = config::load();
    for w in config::validate(&cfg) {
        eprintln!("Warning: {w}");
    }

    let result = match cli.command {
        Command::Login { api_key } => commands::login(api_url, api_key, fmt),
        Command::Status => commands::status(api_url, fmt),
        Command::Doctor => commands::doctor(api_url, fmt),
        Command::Metrics { kind } => commands::metrics(api_url, metric_path(kind), fmt),
        Command::Completion { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "nods", &mut std::io::stdout());
            Ok(())
        }
        Command::Configs(sub) => match sub {
            ConfigsCmd::List => commands::configs_list(api_url, fmt),
            ConfigsCmd::Show { id } => commands::configs_show(api_url, &id, fmt),
            ConfigsCmd::Validate { id } => commands::configs_validate(api_url, &id, fmt),
            ConfigsCmd::Delete { id } => commands::configs_delete(api_url, &id, fmt),
        },
        Command::Sessions(sub) => match sub {
            SessionsCmd::List { status, limit } => {
                commands::sessions_list(api_url, status.as_deref(), limit, fmt)
            }
            SessionsCmd::Show { id } => commands::sessions_show(api_url, &id, fmt),
            SessionsCmd::Launch { config_id } => commands::sessions_launch(api_url, &config_id, fmt),
            SessionsCmd::Kill { id } => commands::sessions_kill(api_url, &id, fmt),
        },
        Command::Templates(sub) => match sub {
            TemplatesCmd::List => commands::templates_list(api_url, fmt),
            TemplatesCmd::Show { id } => commands::templates_show(api_url, &id, fmt),
        },
        Command::Events { r#type, limit } => commands::events(api_url, r#type.as_deref(), limit, fmt),
        Command::Resources => commands::resources(api_url, fmt),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(e.code);
    }
    std::process::exit(exit_codes::OK);
}

fn metric_path(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Usage => "/v1/analytics/usage",
        MetricKind::Performance => "/v1/analytics/performance",
        MetricKind::Backends => "/v1/analytics/backends",
        MetricKind::Daily => "/v1/analytics/daily",
        MetricKind::Errors => "/v1/analytics/errors",
    }
}
