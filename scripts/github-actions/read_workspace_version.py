import os
import re
from pathlib import Path

cargo_toml = Path("Cargo.toml").read_text(encoding="utf-8")
match = re.search(
    r'\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"',
    cargo_toml,
    re.MULTILINE,
)
if not match:
    raise SystemExit("failed to read workspace version")

with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as handle:
    handle.write(f"version={match.group(1)}\n")
