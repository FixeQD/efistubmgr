//! efistubmgr - manage EFISTUB boot entries via efivar/gpt crates

use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn usage(prog: &str) -> ! {
    eprintln!("usage: {prog} list");
    eprintln!(
        "       {prog} create <esp-mount-point> <loader-path-on-esp> <description> <cmdline> [--timestamp <ts>]"
    );
    eprintln!("              (if --timestamp omitted, current time is used)");
    eprintln!("       {prog} delete <num-hex>");
    std::process::exit(2);
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
    }

    let mut mgr = efivar::system();

    let res: eros::Result<()> = (|| {
        match args[1].as_str() {
            "list" => {
                let _guard = lock::acquire_shared()?;
                try_cmd_list(mgr.as_ref())?
            }
            "create" => {
                let mut positional: Vec<String> = Vec::new();
                let mut timestamp_opt: Option<i64> = None;
                let mut i = 2;
                while i < args.len() {
                    let a = &args[i];
                    if a == "--timestamp" {
                        if i + 1 >= args.len() {
                            return Err(eros::error!("--timestamp requires value"));
                        }
                        let v: i64 = args[i + 1].parse::<i64>().map_err(|e| {
                            eros::error!(e)
                                .context(format!("parsing --timestamp {:?} as i64", args[i + 1]))
                        })?;
                        timestamp_opt = Some(v);
                        i += 2;
                    } else if let Some(val) = a.strip_prefix("--timestamp=") {
                        let v: i64 = val.parse::<i64>().map_err(|e| {
                            eros::error!(e).context(format!("parsing --timestamp {val:?} as i64"))
                        })?;
                        timestamp_opt = Some(v);
                        i += 1;
                    } else if a == "-t" {
                        if i + 1 >= args.len() {
                            return Err(eros::error!("-t requires value"));
                        }
                        let v: i64 = args[i + 1].parse::<i64>().map_err(|e| {
                            eros::error!(e).context(format!("parsing -t {:?} as i64", args[i + 1]))
                        })?;
                        timestamp_opt = Some(v);
                        i += 2;
                    } else {
                        positional.push(a.clone());
                        i += 1;
                    }
                }
                // backward compat: 5 positional where last is timestamp
                if positional.len() == 5 && timestamp_opt.is_none() {
                    let ts_str = positional.pop().unwrap();
                    let v: i64 = ts_str.parse::<i64>().map_err(|e| {
                        eros::error!(e).context(format!("parsing timestamp {:?} as i64", ts_str))
                    })?;
                    timestamp_opt = Some(v);
                }
                if positional.len() != 4 {
                    usage(&args[0]);
                }
                let timestamp = timestamp_opt.unwrap_or_else(current_timestamp);
                let _guard = lock::acquire_exclusive()?;
                let id = try_cmd_create(
                    mgr.as_mut(),
                    &positional[0],
                    &positional[1],
                    &positional[2],
                    &positional[3],
                    timestamp,
                )?;
                println!("{id:04X}");
            }
            "delete" => {
                if args.len() != 3 {
                    usage(&args[0]);
                }
                let id = u16::from_str_radix(&args[2], 16).map_err(|e| {
                    eros::error!(e).context(format!("parsing Boot id {:?} as hex u16", args[2]))
                })?;
                let _guard = lock::acquire_exclusive()?;
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
