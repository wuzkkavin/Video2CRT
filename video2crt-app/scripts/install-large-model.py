#!/usr/bin/env python3
"""Install the optional high-quality model selected in the Windows installer.

This script is invoked only by the NSIS installer.  The desktop application's
runtime UI deliberately does not expose model selection.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

from huggingface_hub import snapshot_download


MODEL_VERSION = 1
MODELS = {
    "asr": {
        "repo_id": "mobiuslabsgmbh/faster-whisper-large-v3-turbo",
        "revision": "0a363e9161cbc7ed1431c9597a8ceaf0c4f78fcf",
        "files": ["config.json", "model.bin", "tokenizer.json", "vocabulary.txt"],
    },
    "translation": {
        "repo_id": "jncraton/m2m100_1.2B-ct2-int8",
        "revision": "e50078df6be13a88592b70ea42f4d74c2082e448",
        "files": ["config.json", "model.bin", "shared_vocabulary.json", "sentencepiece.bpe.model"],
    },
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: install-large-model.py <destination>", file=sys.stderr)
        return 2
    root = Path(sys.argv[1]).expanduser().resolve()
    records: dict[str, list[dict[str, object]]] = {}
    try:
        for kind, spec in MODELS.items():
            destination = root / kind
            destination.mkdir(parents=True, exist_ok=True)
            snapshot_download(
                repo_id=spec["repo_id"],
                revision=spec["revision"],
                allow_patterns=spec["files"],
                local_dir=str(destination),
                token=False,
            )
            entries: list[dict[str, object]] = []
            for filename in spec["files"]:
                path = destination / filename
                if not path.is_file():
                    raise FileNotFoundError(path)
                entries.append({"path": filename, "bytes": path.stat().st_size, "sha256": sha256(path)})
            records[kind] = entries
        manifest = {"version": MODEL_VERSION, **records}
        temporary = root / "manifest.json.part"
        temporary.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
        temporary.replace(root / "manifest.json")
        print("高品質模型安裝完成。")
        return 0
    except Exception as exc:  # noqa: BLE001 - installer must return a useful exit code
        print(f"高品質模型安裝失敗：{exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
