import json
import os
import subprocess
from typing import Any


SUPPORTED_TARGETS: list[dict[str, Any]] = [
    # Linux GNU targets via cross
    {"target": "aarch64-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "armv7-unknown-linux-gnueabihf", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "i686-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "loongarch64-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "powerpc64le-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "riscv64gc-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "s390x-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "x86_64-unknown-linux-gnu", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    # Linux musl targets via cross
    {"target": "aarch64-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "armv7-unknown-linux-musleabihf", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "i686-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "loongarch64-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "powerpc64le-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "riscv64gc-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "s390x-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    {"target": "x86_64-unknown-linux-musl", "runner": "ubuntu-latest", "archive": "tar.gz", "builder": "cross"},
    # macOS native targets
    {"target": "aarch64-apple-darwin", "runner": "macos-15", "archive": "tar.gz", "builder": "cargo"},
    {"target": "x86_64-apple-darwin", "runner": "macos-15-intel", "archive": "tar.gz", "builder": "cargo"},
    # Windows MSVC targets
    {"target": "aarch64-pc-windows-msvc", "runner": "windows-11-arm", "archive": "zip", "builder": "cargo"},
    {"target": "i686-pc-windows-msvc", "runner": "windows-2025", "archive": "zip", "builder": "cargo"},
    {"target": "x86_64-pc-windows-msvc", "runner": "windows-2025", "archive": "zip", "builder": "cargo"},
]


def main() -> None:
    target_list = subprocess.run(
        ["rustc", "--print", "target-list"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    known_targets = set(target_list)

    matrix = {
        "include": [
            entry
            for entry in SUPPORTED_TARGETS
            if entry["target"] in known_targets
        ]
    }

    output = json.dumps(matrix)
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as handle:
            handle.write(f"matrix={output}\n")
    else:
        print(output)


if __name__ == "__main__":
    main()
