<div align="center">
<h1>efistubmgr</h1>
</div>

Minimal EFISTUB manager written in Rust. It creates and manages UEFI boot entries pointing directly at a Linux kernel EFI stub, bypassing external bootloaders. 

Designed for NixOS/finix setups where system rebuilds generate new kernel images on every run and you need deterministic boot entry management without parsing raw `efibootmgr` output.

> [!INFO]
> `efistubmgr` is a primitive, low-level CLI tool, not a full bootloader framework. It solely manages NVRAM entries. You need to write your own orchestration script (with external bootloader hook) to copy kernels/initrds to the ESP, manage GC/pruning of old files, and trigger `efistubmgr`.

## Overview

Interacting with `efibootmgr` via automation is brittle. `efistubmgr` embeds a custom timestamp inside the `optional_data` array of each UEFI entry it generates. This allows it to identify its own entries, maintain chronological sorting, and manage `BootOrder` programmatically.

Entries created by other tools (Windows Boot Manager, shim, standard system entries) remain completely untouched.

## Features

* `create` - Allocates a new `Boot####` entry on the target ESP, appends the cmdline and timestamp metadata, then prepends its ID to `BootOrder`.
* `delete` - Removes the specified `Boot####` entry from NVRAM and cleans references in `BootOrder`.
* `list` - Parses NVRAM and outputs `efistubmgr`-managed entries sorted newest-first.

## Output Format & Returns

`efistubmgr` is designed to be easily parsed inside scripts:

* `create` - Returns the assigned entry ID (`Boot####` without the `Boot` prefix, e.g. `0001`).
* `list` - Returns lines formatted as `<entry_id> <creation_timestamp>` (e.g. `0001 1787820063`), sorted newest-first.
* `delete` - Returns nothing on success (exit status 0).

## Nix Flake Usage

Add `efistubmgr` to your `flake.nix` inputs:

```nix
inputs = {
  nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  
  efistubmgr = {
    url = "github:FixeQD/efistubmgr";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

Then expose the package via `overlays` in your system configuration:

```nix
nixpkgs.overlays = [
  (final: prev: {
    efistubmgr = efistubmgr.packages.${system}.default;
  })
];
```

## Usage

```bash
efistubmgr create <esp-mount-point> <loader-path-on-esp> <description> <cmdline> [--timestamp <ts>]
efistubmgr delete <id-hex>
efistubmgr list
```

Run `efistubmgr <subcommand> --help` for full flag descriptions.

## Example Integration Script

<details>
<summary>Click to view a complete NixOS/finix install hook script</summary>

```nix
efistubHook = pkgs.writeShellScript "efistub-install" ''
  set -euo pipefail

  # ── Paths & metadata ──────────────────────────────────────────────────────

  BOOTSPEC="$1/boot.json"
  BOOT_DIR="${efiMount}/EFI/nixos"
  EFISTUBMGR="${pkgs.efistubmgr}/bin/efistubmgr"

  TIMESTAMP=$(${pkgs.coreutils}/bin/date +%s)
  HUMAN_DATE=$(
    ${pkgs.coreutils}/bin/date -d "@$TIMESTAMP" '+%Y-%m-%d %H:%M:%S %Z'
  )

  KERNEL_PATH="$BOOT_DIR/kernel-$TIMESTAMP.efi"
  INITRD_PATH="$BOOT_DIR/initrd-$TIMESTAMP"

  # ── Helpers ───────────────────────────────────────────────────────────────

  get_rev() {
    local target="$1"
    local path="$2"
    local link

    for link in /nix/var/nix/profiles/system-*-link; do
      [ -e "$link" ] || continue

      case "$(readlink "$link")" in
        "$target"|"$path")
          printf '%s\n' "$link" |
            ${pkgs.gnugrep}/bin/grep -oE 'system-[0-9]+-link' |
            ${pkgs.gnugrep}/bin/grep -oE '[0-9]+' |
            ${pkgs.gawk}/bin/awk '{print "rev. " $1}'
          return
          ;;
      esac
    done
  }

  is_kept_timestamp() {
    local needle="$1"

    for ts in "''${KEEP_TIMESTAMPS[@]}"; do
      [ "$ts" = "$needle" ] && return 0
    done

    return 1
  }

  is_seen_timestamp() {
    local needle="$1"

    for ts in "''${ORPHAN_TIMESTAMPS[@]}"; do
      [ "$ts" = "$needle" ] && return 0
    done

    return 1
  }

  # ── Validate & read bootspec ──────────────────────────────────────────────

  [ -r "$BOOTSPEC" ] ||
    { echo "ERROR: boot.json not found at $BOOTSPEC"; exit 1; }

  [ -b "${bootDisk}" ] ||
    { echo "ERROR: boot device not found: ${bootDisk}"; exit 1; }

  KERNEL=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".kernel' "$BOOTSPEC")
  INITRD=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".initrd' "$BOOTSPEC")
  INIT=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".init' "$BOOTSPEC")
  PARAMS=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".kernelParams | join(" ")' "$BOOTSPEC")
  LABEL=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".label // empty' "$BOOTSPEC")
  TOPLEVEL=$(${pkgs.jq}/bin/jq -r '."org.nixos.bootspec.v1".toplevel // empty' "$BOOTSPEC")

  [ -z "$LABEL" ] || [ "$LABEL" = "null" ] && LABEL="Finix"
  [ -z "$TOPLEVEL" ] && TOPLEVEL="$1"

  REV=$(get_rev "$TOPLEVEL" "$1")
  DESCRIPTION="$LABEL''${REV:+ $REV} ❖ $HUMAN_DATE"

  # ── Install kernel & initrd ───────────────────────────────────────────────

  mkdir -p "$BOOT_DIR"

  echo "==> Installing kernel and initrd (timestamp: $TIMESTAMP)"
  install -m 0644 "$KERNEL" "$KERNEL_PATH"
  install -m 0644 "$INITRD" "$INITRD_PATH"

  # ── Secure Boot ───────────────────────────────────────────────────────────

  if [ -d /etc/secureboot/keys ]; then
    echo "==> sbctl: signing kernel"
    ${pkgs.sbctl}/bin/sbctl sign "$KERNEL_PATH" || \
      echo "WARNING: sbctl signing failed, but continuing (Secure Boot may not work)"
  fi

  # ── Create NVRAM entry ────────────────────────────────────────────────────

  echo "==> Creating new primary entry: $DESCRIPTION (timestamp $TIMESTAMP)"

  NEW_ID=$("$EFISTUBMGR" create "${efiMount}" \
    '\EFI\nixos\kernel-'"$TIMESTAMP"'.efi' \
    "$DESCRIPTION" \
    "initrd=\EFI\nixos\initrd-$TIMESTAMP init=$INIT $PARAMS" \
    --timestamp "$TIMESTAMP")

  echo "==> Created Boot$NEW_ID"

  # ── Keep newest generations ───────────────────────────────────────────────

  mapfile -t GENERATIONS < <("$EFISTUBMGR" list)
  echo "==> ''${#GENERATIONS[@]} finix generation(s) in NVRAM"

  KEEP_TIMESTAMPS=()
  for line in "''${GENERATIONS[@]:0:${toString keepGenerations}}"; do
    KEEP_TIMESTAMPS+=(
      "$(${pkgs.gawk}/bin/awk '{print $2}' <<<"$line")"
    )
  done

  declare -A PRUNE_IDS

  if [ "''${#GENERATIONS[@]}" -gt "${toString keepGenerations}" ]; then
    for line in "''${GENERATIONS[@]:${toString keepGenerations}}"; do
      id=$(${pkgs.gawk}/bin/awk '{print $1}' <<<"$line")
      ts=$(${pkgs.gawk}/bin/awk '{print $2}' <<<"$line")

      "$EFISTUBMGR" delete "$id"
      PRUNE_IDS["$ts"]="$id"
    done
  fi

  # ── Remove orphaned ESP files ─────────────────────────────────────────────

  ORPHAN_TIMESTAMPS=()

  for f in "$BOOT_DIR"/kernel-*.efi "$BOOT_DIR"/initrd-*; do
    [ -e "$f" ] || continue

    base=$(basename "$f")
    file_ts="''${base#*-}"
    file_ts="''${file_ts%.efi}"

    if ! is_kept_timestamp "$file_ts" &&
       ! is_seen_timestamp "$file_ts"; then
      ORPHAN_TIMESTAMPS+=("$file_ts")
    fi
  done

  for ts in "''${ORPHAN_TIMESTAMPS[@]}"; do
    if [ -n "''${PRUNE_IDS[$ts]:-}" ]; then
      echo "==> Removing orphaned ESP files kernel-$ts.efi + initrd-$ts" \
        "(ts $ts, removed matching Boot''${PRUNE_IDS[$ts]} NVRAM entry)"
    else
      echo "==> Removing orphaned ESP files kernel-$ts.efi + initrd-$ts" \
        "(ts $ts, no matching NVRAM entry)"
    fi

    rm -f "$BOOT_DIR/kernel-$ts.efi" "$BOOT_DIR/initrd-$ts"
  done

  echo "==> EFISTUB setup complete"
'';
```
</details>

## Build

```bash
nix build
```

Or via Cargo:

```bash
cargo build --release
```

## Technical Notes

* Boot parameters are serialized in UTF-16LE encoding to align with Linux EFI stub parsing requirements.
* Operations (`create`, `delete`, `list`) acquire an exclusive lock on `/run/efistubmgr.lock` to prevent race conditions during parallel nix rebuilds.
* Direct NVRAM manipulation contains inherent risks. `delete` will remove any valid ID supplied to it, even if created externally. Validate target paths and arguments before execution.

## License

GPL-3.0-only
