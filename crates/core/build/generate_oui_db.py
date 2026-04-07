#!/usr/bin/env python3
"""Manual IEEE OUI CSV -> Rust lookup table generator.

This script is intentionally manual. Normal `cargo build` must not perform
network fetches. Use one of:

1) Local CSV input (preferred for full DB):
   python3 crates/core/build/generate_oui_db.py \
     --input-csv /path/to/oui.csv \
     --output-rust crates/core/src/scanner/oui_db.rs

2) Manual download + generate:
   python3 crates/core/build/generate_oui_db.py \
     --download \
     --output-rust crates/core/src/scanner/oui_db.rs

3) Offline bootstrap seed (incomplete, for constrained environments):
   python3 crates/core/build/generate_oui_db.py \
     --use-bootstrap \
     --output-rust crates/core/src/scanner/oui_db.rs
"""

from __future__ import annotations

import argparse
import csv
import pathlib
import re
import tempfile
import urllib.request

DEFAULT_IEEE_CSV_URL = "https://standards-oui.ieee.org/oui/oui.csv"
REPO_ROOT = pathlib.Path(__file__).resolve().parents[4]
DEFAULT_OUTPUT = REPO_ROOT / "new-arch" / "crates" / "core" / "src" / "scanner" / "oui_db.rs"

# Keep this tiny and curated; it is only for offline bootstrap generation.
BOOTSTRAP_ENTRIES: list[tuple[int, str]] = [
    (0x00044B, "NVIDIA CORPORATION"),
    (0x0021CC, "Lenovo Group Limited"),
    (0x2CCF67, "Raspberry Pi Trading Ltd"),
    (0x3C5282, "Hewlett Packard"),
    (0x3C6D66, "NVIDIA CORPORATION"),
    (0x48B02D, "NVIDIA CORPORATION"),
    (0xA45E60, "Apple, Inc."),
    (0xAC91A1, "D-Robotics"),
    (0xB827EB, "Raspberry Pi Trading Ltd"),
    (0xD83ADD, "Raspberry Pi Trading Ltd"),
    (0xDCA632, "Raspberry Pi Trading Ltd"),
    (0xE45F01, "Raspberry Pi Trading Ltd"),
    (0xF01898, "Apple, Inc."),
    (0xF8B156, "Dell Inc."),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate checked-in Rust OUI lookup table from IEEE CSV."
    )
    parser.add_argument(
        "--input-csv",
        type=pathlib.Path,
        help="Path to IEEE OUI CSV (e.g. oui.csv).",
    )
    parser.add_argument(
        "--download",
        action="store_true",
        help="Manually download IEEE OUI CSV before generating.",
    )
    parser.add_argument(
        "--download-url",
        default=DEFAULT_IEEE_CSV_URL,
        help=f"Download URL used with --download (default: {DEFAULT_IEEE_CSV_URL}).",
    )
    parser.add_argument(
        "--use-bootstrap",
        action="store_true",
        help="Use a tiny built-in seed database when full CSV is unavailable.",
    )
    parser.add_argument(
        "--output-rust",
        type=pathlib.Path,
        default=DEFAULT_OUTPUT,
        help=f"Output Rust module path (default: {DEFAULT_OUTPUT}).",
    )
    return parser.parse_args()


def resolve_input_csv(args: argparse.Namespace) -> tuple[pathlib.Path | None, str]:
    if args.input_csv is not None:
        return args.input_csv, f"local CSV: {args.input_csv}"

    if args.download:
        tmp_path = pathlib.Path(tempfile.gettempdir()) / "ieee-oui.csv"
        urllib.request.urlretrieve(args.download_url, tmp_path)
        return tmp_path, f"downloaded CSV: {args.download_url}"

    if args.use_bootstrap:
        return None, "bootstrap seed (incomplete)"

    raise SystemExit(
        "No input source selected. Use --input-csv, --download, or --use-bootstrap."
    )


def parse_hex_prefix(raw_assignment: str) -> int | None:
    compact = re.sub(r"[^0-9A-Fa-f]", "", raw_assignment or "")
    if len(compact) != 6:
        return None
    try:
        return int(compact, 16)
    except ValueError:
        return None


def normalize_vendor_name(raw_vendor: str) -> str:
    cleaned = " ".join((raw_vendor or "").strip().split())
    if not cleaned:
        return ""
    return cleaned


def parse_ieee_csv(csv_path: pathlib.Path) -> list[tuple[int, str]]:
    if not csv_path.exists():
        raise SystemExit(f"Input CSV not found: {csv_path}")

    entries: dict[int, str] = {}
    with csv_path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise SystemExit(f"CSV has no headers: {csv_path}")

        for row in reader:
            assignment = row.get("Assignment", "")
            prefix = parse_hex_prefix(assignment)
            if prefix is None:
                continue

            vendor_name = normalize_vendor_name(row.get("Organization Name", ""))
            if not vendor_name:
                continue

            # Keep first-seen vendor per prefix for deterministic output.
            entries.setdefault(prefix, vendor_name)

    if not entries:
        raise SystemExit(
            f"No valid OUI entries parsed from {csv_path}. Check CSV format/content."
        )

    return sorted(entries.items(), key=lambda item: item[0])


def rust_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def format_prefix(prefix: int) -> str:
    b1 = (prefix >> 16) & 0xFF
    b2 = (prefix >> 8) & 0xFF
    b3 = prefix & 0xFF
    return f"[0x{b1:02X}, 0x{b2:02X}, 0x{b3:02X}]"


def render_rust_module(entries: list[tuple[int, str]], source_label: str) -> str:
    lines: list[str] = [
        "// This file is @generated by crates/core/build/generate_oui_db.py.",
        "// Do not edit by hand. Regenerate with the script above.",
        f"// Source: {source_label}",
        "",
        "const OUI_VENDOR_TABLE: &[([u8; 3], &str)] = &[",
    ]

    for prefix, vendor in entries:
        lines.append(f'    ({format_prefix(prefix)}, "{rust_escape(vendor)}"),')

    lines.extend(
        [
            "];",
            "",
            "pub fn lookup_vendor_name(prefix: [u8; 3]) -> Option<&'static str> {",
            "    let index = OUI_VENDOR_TABLE",
            "        .binary_search_by_key(&prefix, |(candidate, _)| *candidate)",
            "        .ok()?;",
            "    Some(OUI_VENDOR_TABLE[index].1)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::lookup_vendor_name;",
            "",
            "    #[test]",
            "    fn returns_none_for_unknown_prefix() {",
            "        assert_eq!(lookup_vendor_name([0xDE, 0xAD, 0xBE]), None);",
            "    }",
            "}",
            "",
        ]
    )
    return "\n".join(lines)


def write_output(path: pathlib.Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def main() -> int:
    args = parse_args()
    csv_path, source_label = resolve_input_csv(args)
    if csv_path is None:
        entries = sorted(BOOTSTRAP_ENTRIES, key=lambda item: item[0])
    else:
        entries = parse_ieee_csv(csv_path)

    output_path = args.output_rust.resolve()
    output = render_rust_module(entries, source_label)
    write_output(output_path, output)

    print(f"Wrote {len(entries)} OUI entries to {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
