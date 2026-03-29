import os
import re
from pathlib import Path

version = os.environ["VERSION"]
cargo_toml_path = Path("Cargo.toml")
cargo_toml = cargo_toml_path.read_text(encoding="utf-8")

workspace_version_pattern = r'(\[workspace\.package\][\s\S]*?^version\s*=\s*")([^"]+)(")'
updated, replacements = re.subn(
    workspace_version_pattern,
    rf"\g<1>{version}\3",
    cargo_toml,
    count=1,
    flags=re.MULTILINE,
)
if replacements != 1:
    raise SystemExit("failed to update workspace version")


def update_internal_dependency(match: re.Match[str]) -> str:
    line = match.group(0)
    if 'path = "crates/' not in line:
        return line
    return re.sub(
        r'(version\s*=\s*")([^"]+)(")',
        rf"\g<1>{version}\3",
        line,
        count=1,
    )


updated = re.sub(
    r"^vs-[^=]+\s*=\s*\{[^}]*\}$",
    update_internal_dependency,
    updated,
    flags=re.MULTILINE,
)

cargo_toml_path.write_text(updated, encoding="utf-8")
