#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import pathlib
import platform
import re
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile


def request_headers(accept: str = "application/vnd.github+json") -> dict[str, str]:
    headers = {
        "Accept": accept,
        "User-Agent": "terminal-platform-ci",
    }
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def download_json(url: str) -> dict:
    request = urllib.request.Request(url, headers=request_headers())
    with urllib.request.urlopen(request) as response:
        return json.load(response)


def download_file(url: str, destination: pathlib.Path) -> None:
    request = urllib.request.Request(
        url,
        headers=request_headers(accept="application/octet-stream"),
    )
    with urllib.request.urlopen(request) as response, destination.open("wb") as output:
        shutil.copyfileobj(response, output)


def candidate_suffixes() -> tuple[str, ...]:
    runner_os = os.environ.get("RUNNER_OS")
    runner_arch = os.environ.get("RUNNER_ARCH")

    if runner_os is None:
        runner_os = {
            "Darwin": "macOS",
            "Linux": "Linux",
            "Windows": "Windows",
        }.get(platform.system(), platform.system())
    if runner_arch is None:
        runner_arch = {
            "x86_64": "X64",
            "AMD64": "X64",
            "arm64": "ARM64",
            "aarch64": "ARM64",
        }.get(platform.machine(), platform.machine())

    arch = {
        "X64": "x86_64",
        "ARM64": "aarch64",
    }.get(runner_arch, runner_arch.lower())

    if runner_os == "Linux":
        return (f"{arch}-unknown-linux-musl.tar.gz",)
    if runner_os == "macOS":
        return (f"{arch}-apple-darwin.tar.gz",)
    if runner_os == "Windows":
        return (f"{arch}-pc-windows-msvc.zip",)

    raise RuntimeError(f"unsupported runner os: {runner_os}")


def select_asset(release: dict) -> dict:
    assets = release.get("assets", [])
    suffixes = candidate_suffixes()

    for prefix in ("zellij-no-web-", "zellij-"):
        for suffix in suffixes:
            for asset in assets:
                name = asset.get("name", "")
                if name.startswith(prefix) and name.endswith(suffix):
                    return asset

    names = ", ".join(asset.get("name", "<unnamed>") for asset in assets)
    raise RuntimeError(f"failed to locate zellij asset for {suffixes}: {names}")


def zellij_version_tuple(release: dict) -> tuple[int, int, int]:
    version_text = str(release.get("tag_name") or release.get("name") or "")
    match = re.search(r"(\d+)\.(\d+)\.(\d+)", version_text)
    if not match:
        raise RuntimeError(f"failed to parse zellij release version from: {version_text!r}")

    return tuple(int(part) for part in match.groups())


def assert_supported_zellij_release(release: dict) -> None:
    version = zellij_version_tuple(release)
    if version < (0, 44, 0):
        formatted = ".".join(str(part) for part in version)
        raise RuntimeError(f"zellij {formatted} is below the v1 minimum 0.44.0")


def extract_binary(archive_path: pathlib.Path, out_dir: pathlib.Path) -> pathlib.Path:
    out_dir.mkdir(parents=True, exist_ok=True)

    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path) as archive:
            for member in archive.namelist():
                if member.endswith("/"):
                    continue
                filename = pathlib.Path(member).name
                if filename not in {"zellij", "zellij.exe"}:
                    continue
                target = out_dir / filename
                with archive.open(member) as source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
                return target
        raise RuntimeError("zellij executable not found in zip archive")

    with tarfile.open(archive_path) as archive:
        for member in archive.getmembers():
            filename = pathlib.Path(member.name).name
            if filename != "zellij":
                continue
            target = out_dir / filename
            source = archive.extractfile(member)
            if source is None:
                continue
            with source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            target.chmod(target.stat().st_mode | stat.S_IEXEC)
            return target

    raise RuntimeError("zellij executable not found in tar archive")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True, help="directory where the zellij binary should land")
    args = parser.parse_args()

    release = download_json("https://api.github.com/repos/zellij-org/zellij/releases/latest")
    assert_supported_zellij_release(release)
    asset = select_asset(release)

    out_dir = pathlib.Path(args.out).resolve()
    with tempfile.TemporaryDirectory(prefix="terminal-platform-zellij-") as temp_dir:
        archive_path = pathlib.Path(temp_dir) / asset["name"]
        download_file(asset["browser_download_url"], archive_path)
        binary_path = extract_binary(archive_path, out_dir)

    print(binary_path.parent)
    return 0


if __name__ == "__main__":
    sys.exit(main())
