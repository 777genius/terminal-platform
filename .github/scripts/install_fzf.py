#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import pathlib
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request
import zipfile

REQUEST_TIMEOUT_SECONDS = 30
REQUEST_ATTEMPTS = 3


def request_headers(accept: str = "application/vnd.github+json") -> dict[str, str]:
    headers = {
        "Accept": accept,
        "User-Agent": "terminal-platform-ci",
    }
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def open_url_with_retries(request: urllib.request.Request, url: str):
    last_error: BaseException | None = None

    for attempt in range(1, REQUEST_ATTEMPTS + 1):
        try:
            return urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS)
        except (TimeoutError, urllib.error.URLError, urllib.error.HTTPError) as error:
            last_error = error
            if attempt == REQUEST_ATTEMPTS:
                break
            time.sleep(attempt)

    raise RuntimeError(
        f"failed to download {url} after {REQUEST_ATTEMPTS} attempts: {last_error}"
    ) from last_error


def download_json(url: str) -> dict:
    request = urllib.request.Request(url, headers=request_headers())
    with open_url_with_retries(request, url) as response:
        return json.load(response)


def download_file(url: str, destination: pathlib.Path) -> None:
    request = urllib.request.Request(
        url,
        headers=request_headers(accept="application/octet-stream"),
    )
    with open_url_with_retries(request, url) as response, destination.open("wb") as output:
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

    if runner_os == "Linux":
        arch = {
            "X64": "amd64",
            "ARM64": "arm64",
        }.get(runner_arch, runner_arch.lower())
        return (f"linux_{arch}.tar.gz",)
    if runner_os == "macOS":
        arch = {
            "X64": "amd64",
            "ARM64": "arm64",
        }.get(runner_arch, runner_arch.lower())
        return (f"darwin_{arch}.tar.gz",)
    if runner_os == "Windows":
        arch = {
            "X64": "amd64",
            "ARM64": "arm64",
        }.get(runner_arch, runner_arch.lower())
        return (f"windows_{arch}.zip",)

    raise RuntimeError(f"unsupported runner os: {runner_os}")


def select_asset(release: dict) -> dict:
    assets = release.get("assets", [])
    suffixes = candidate_suffixes()

    for suffix in suffixes:
        for asset in assets:
            name = asset.get("name", "")
            if name.startswith("fzf-") and name.endswith(suffix):
                return asset

    names = ", ".join(asset.get("name", "<unnamed>") for asset in assets)
    raise RuntimeError(f"failed to locate fzf asset for {suffixes}: {names}")


def extract_binary(archive_path: pathlib.Path, out_dir: pathlib.Path) -> pathlib.Path:
    out_dir.mkdir(parents=True, exist_ok=True)

    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path) as archive:
            for member in archive.namelist():
                if member.endswith("/"):
                    continue
                filename = pathlib.Path(member).name
                if filename != "fzf.exe":
                    continue
                target = out_dir / filename
                with archive.open(member) as source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
                return target
        raise RuntimeError("fzf executable not found in zip archive")

    with tarfile.open(archive_path) as archive:
        for member in archive.getmembers():
            filename = pathlib.Path(member.name).name
            if filename != "fzf":
                continue
            target = out_dir / filename
            source = archive.extractfile(member)
            if source is None:
                continue
            with source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            target.chmod(target.stat().st_mode | stat.S_IEXEC)
            return target

    raise RuntimeError("fzf executable not found in archive")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True, help="directory where the fzf binary should land")
    args = parser.parse_args()

    release = download_json("https://api.github.com/repos/junegunn/fzf/releases/latest")
    asset = select_asset(release)

    out_dir = pathlib.Path(args.out).resolve()
    with tempfile.TemporaryDirectory(prefix="terminal-platform-fzf-") as temp_dir:
        archive_path = pathlib.Path(temp_dir) / asset["name"]
        download_file(asset["browser_download_url"], archive_path)
        binary_path = extract_binary(archive_path, out_dir)

    print(binary_path.parent)
    return 0


if __name__ == "__main__":
    sys.exit(main())
