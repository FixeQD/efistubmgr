//! efistubmgr - manage EFISTUB boot entries through the `efivar` and `gpt` crates instead of shelling out to efibootmgr/blkid/lsblk

use std::env;
use std::path::Path;
use std::process::ExitCode;

use efivar::efi::Variable;
use efivar::VarManager;
use eros::Context;

pub mod boot;
pub mod mount;
pub mod nvram;
pub mod timestamp;

#[cfg(test)]
pub mod testtoolkit;

pub const DESC_PREFIX: &str = "Efistub";

pub fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("{}: {msg}", env!("CARGO_PKG_NAME"));
    std::process::exit(1);
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
    optional_data: &str,
    timestamp: i64,
) -> eros::Result<u16> {
    let hard_drive = mount::try_build_hard_drive(Path::new(esp_mount_point))?;
    boot::try_cmd_create_with_hard_drive(mgr, hard_drive, loader_path, optional_data, timestamp)
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

fn usage(prog: &str) -> ! {
    eprintln!("usage: {prog} list");
    eprintln!(
        "       {prog} create <esp-mount-point> <loader-path-on-esp> <optional-data> <timestamp>"
    );
    eprintln!("       {prog} delete <num-hex>");
    std::process::exit(2);
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
    }

    let mut mgr = efivar::system();

    let res: eros::Result<()> = (|| {
        match args[1].as_str() {
            "list" => try_cmd_list(mgr.as_ref())?,
            "create" => {
                if args.len() != 6 {
                    usage(&args[0]);
                }
                let timestamp: i64 = args[5].parse::<i64>().map_err(|e| {
                    eros::error!(e).context(format!("parsing timestamp {:?} as i64", args[5]))
                })?;
                let id = try_cmd_create(mgr.as_mut(), &args[2], &args[3], &args[4], timestamp)?;
                println!("{id:04X}");
            }
            "delete" => {
                if args.len() != 3 {
                    usage(&args[0]);
                }
                let id = u16::from_str_radix(&args[2], 16).map_err(|e| {
                    eros::error!(e).context(format!("parsing Boot id {:?} as hex u16", args[2]))
                })?;
                try_cmd_delete(mgr.as_mut(), id)?;
            }
            _ => usage(&args[0]),
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
