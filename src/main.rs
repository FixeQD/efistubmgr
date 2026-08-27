//! efistubmgr - manage EFISTUB boot entries via efivar/gpt crates

use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use efivar::efi::Variable;
use efivar::VarManager;
use eros::Context;

pub mod boot;
pub mod lock;
pub mod metadata;
pub mod mount;
pub mod nvram;

#[cfg(test)]
pub mod testtoolkit;

pub fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("{}: {msg}", env!("CARGO_PKG_NAME"));
    std::process::exit(1);
}

#[derive(Parser)]
#[command(name = "efistubmgr", about = "Script-friendly EFISTUB manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    List,
    Create {
        esp_mount_point: String,
        loader_path: String,
        description: String,
        cmdline: String,
        #[arg(long, short = 't')]
        timestamp: Option<i64>,
    },
    Delete {
        #[arg(value_parser = parse_hex_id)]
        id: u16,
    },
}

fn parse_hex_id(s: &str) -> Result<u16, String> {
    u16::from_str_radix(s, 16).map_err(|e| format!("invalid hex id {s:?}: {e}"))
}

fn try_cmd_list(mgr: &dyn VarManager) -> eros::Result<()> {
    for g in nvram::try_list_generations(mgr)? {
        println!("{:04X} {}", g.id, g.ts);
    }
    Ok(())
}

fn try_cmd_create(
    mgr: &mut dyn VarManager,
    esp_mount_point: &str,
    loader_path: &str,
    description: &str,
    cmdline: &str,
    timestamp: i64,
) -> eros::Result<u16> {
    let hard_drive = mount::try_build_hard_drive(Path::new(esp_mount_point))?;
    boot::try_cmd_create_with_hard_drive(
        mgr,
        hard_drive,
        loader_path,
        description,
        cmdline,
        timestamp,
    )
}

fn try_cmd_delete(mgr: &mut dyn VarManager, id: u16) -> eros::Result<()> {
    mgr.delete(&Variable::new(&format!("Boot{id:04X}")))
        .with_context(|| format!("deleting Boot{id:04X}"))?;

    let mut order = match mgr.get_boot_order() {
        Ok(order) => order,
        Err(efivar::Error::VarNotFound { .. }) => Vec::new(),
        Err(e) => return Err(eros::error!(e).context("reading BootOrder")),
    };

    let before = order.len();
    order.retain(|&x| x != id);
    if order.len() != before {
        mgr.set_boot_order(order).context("writing BootOrder")?;
    }
    Ok(())
}

#[cfg(test)]
fn cmd_delete(mgr: &mut dyn VarManager, id: u16) {
    try_cmd_delete(mgr, id).unwrap_or_else(|e| die(format!("{e}")))
}

fn timestamp_from(now: SystemTime) -> eros::Result<i64> {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before the Unix epoch, pass --timestamp explicitly")?
        .as_secs();
    Ok(secs as i64)
}

fn current_timestamp() -> eros::Result<i64> {
    timestamp_from(SystemTime::now())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut mgr = efivar::system();

    let res: eros::Result<()> = (|| {
        match cli.command {
            Command::List => {
                let _guard = lock::acquire_shared()?;
                try_cmd_list(mgr.as_ref())?;
            }
            Command::Create {
                esp_mount_point,
                loader_path,
                description,
                cmdline,
                timestamp,
            } => {
                let timestamp = match timestamp {
                    Some(v) => v,
                    None => current_timestamp()?,
                };
                let _guard = lock::acquire_exclusive()?;
                let id = try_cmd_create(
                    mgr.as_mut(),
                    &esp_mount_point,
                    &loader_path,
                    &description,
                    &cmdline,
                    timestamp,
                )?;
                println!("{id:04X}");
            }
            Command::Delete { id } => {
                let _guard = lock::acquire_exclusive()?;
                try_cmd_delete(mgr.as_mut(), id)?;
            }
        }
        Ok(())
    })();

    if let Err(e) = res {
        eprintln!("{}: {e}", env!("CARGO_PKG_NAME"));
        if std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!("{e:?}");
        }
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
