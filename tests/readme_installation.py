"""Read-only check: python3 tests/readme_installation.py (requires gh)."""

import json
from pathlib import Path
import re
import subprocess


root = Path(__file__).resolve().parents[1]
readme = (root / "README.md").read_text()
url, tag, asset = re.search(
    r"(https://github.com/mi2428/clockping/releases/download/([^/\s]+)/([^\s]+))",
    readme,
).groups()
release = json.loads(subprocess.check_output(
    ["gh", "release", "view", tag, "--repo", "mi2428/clockping",
     "--json", "tagName,assets"], text=True,
))
assert release["tagName"] == tag
assert any(item["name"] == asset and item["url"] == url for item in release["assets"])
assert not re.search(r"/(?:Users|home)/[^/\s]+", readme)
assert 'INSTALL_BINDIR="$HOME/bin"' in readme
help_text = subprocess.check_output(["make", "help", "HOME=/example-home"], cwd=root, text=True)
help_text = re.sub(r"\x1b\[[0-9;]*m", "", help_text).replace("/example-home", "$HOME")
install_help = next(line.split() for line in help_text.splitlines() if line.strip().startswith("INSTALL_BINDIR "))
assert install_help in [line.split() for line in readme.splitlines()]
dry_run = subprocess.check_output(
    ["make", "-n", "install", "INSTALL_BINDIR=/example/bin"], cwd=root, text=True,
)
assert '"/example/bin/clockping"' in dry_run
print(f"README installation: verified {tag}/{asset}, portable default and override")
