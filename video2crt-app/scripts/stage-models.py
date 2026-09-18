#!/usr/bin/env python3
"""Stage the pinned offline model snapshots used by the desktop installer."""
from __future__ import annotations

import os
from pathlib import Path

from huggingface_hub import snapshot_download


ROOT = Path(os.environ["VIDEO2CRT_MODEL_ROOT"])
MODELS = {
    "asr": {
        "repo_id": "Systran/faster-whisper-small",
        "revision": "536b0662742c02347bc0e980a01041f333bce120",
        "allow_patterns": [
            "config.json",
            "model.bin",
            "tokenizer.json",
            "vocabulary.txt",
        ],
    },
    "translation": {
        "repo_id": "jncraton/m2m100_418M-ct2-int8",
        "revision": "7c1b2620a4e58dacecbd8bf89cfd6da7eb9eb7b0",
        "allow_patterns": [
            "config.json",
            "model.bin",
            "shared_vocabulary.json",
            "sentencepiece.bpe.model",
        ],
    },
}


def stage(name: str, options: dict[str, object]) -> None:
    destination = ROOT / name
    destination.mkdir(parents=True, exist_ok=True)
    try:
        snapshot_download(
            repo_id=str(options["repo_id"]),
            revision=str(options["revision"]),
            allow_patterns=list(options["allow_patterns"]),
            token=False,
            local_files_only=True,
            local_dir=str(destination),
        )
    except Exception:
        snapshot_download(
            repo_id=str(options["repo_id"]),
            revision=str(options["revision"]),
            allow_patterns=list(options["allow_patterns"]),
            token=False,
            local_dir=str(destination),
        )
    missing = [
        file_name for file_name in options["allow_patterns"]
        if not (destination / str(file_name)).is_file()
    ]
    if missing:
        raise RuntimeError(f"{name} model is incomplete: {', '.join(missing)}")
    print(f"[OK] staged {name} model in {destination}")


for model_name, model_options in MODELS.items():
    stage(model_name, model_options)
