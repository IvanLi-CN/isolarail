#!/usr/bin/env python3
"""Generate score JSON, MIDI, and WAV previews from the HTML tone data."""

from __future__ import annotations

import argparse
import html.parser
import json
import shutil
import subprocess
import sys
from pathlib import Path


class ToneDataParser(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.in_tone_data = False
        self.parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag != "script":
            return
        attr_map = dict(attrs)
        if attr_map.get("id") == "tone-data":
            self.in_tone_data = True

    def handle_endtag(self, tag: str) -> None:
        if tag == "script" and self.in_tone_data:
            self.in_tone_data = False

    def handle_data(self, data: str) -> None:
        if self.in_tone_data:
            self.parts.append(data)


def load_tone_data(index_html: Path) -> dict:
    parser = ToneDataParser()
    parser.feed(index_html.read_text(encoding="utf-8"))
    raw = "".join(parser.parts).strip()
    if not raw:
        raise ValueError(f"tone-data JSON not found in {index_html}")
    return json.loads(raw)


def slug(value: str) -> str:
    out = []
    for char in value.lower():
        if char.isalnum():
            out.append(char)
        else:
            out.append("-")
    return "-".join("".join(out).split("-")).strip("-")


def score_for(data: dict, events: list[dict]) -> dict:
    audio = data["audio"]
    return {
        "tempo_bpm": 120,
        "audio": {
            "waveform": audio["waveform"],
            "sample_rate_hz": 44100,
            "volume": audio["volume"],
            "fade_ms": audio["fadeMs"],
        },
        "midi": {
            "channel": 0,
            "program": 80,
            "velocity": 96,
        },
        "events": events,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--index", type=Path, default=Path(__file__).with_name("index.html"))
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).with_name("out"))
    parser.add_argument(
        "--generator",
        type=Path,
        default=Path.home() / ".codex/skills/buzzer-audio-preview/scripts/buzzer_preview.py",
    )
    args = parser.parse_args()

    if not args.generator.exists():
        print(f"missing buzzer preview generator: {args.generator}", file=sys.stderr)
        return 2

    data = load_tone_data(args.index)
    scores_dir = args.out_dir / "scores"
    media_dir = args.out_dir / "media"
    if args.out_dir.exists():
        shutil.rmtree(args.out_dir)
    scores_dir.mkdir(parents=True)
    media_dir.mkdir(parents=True)

    generated = 0
    for tone in data["tones"]:
        for candidate in tone["candidates"]:
            name = f"{slug(tone['id'])}__{slug(candidate['id'])}"
            score_path = scores_dir / f"{name}.json"
            score_path.write_text(
                json.dumps(score_for(data, candidate["events"]), ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            subprocess.run(
                [
                    sys.executable,
                    str(args.generator),
                    "--in",
                    str(score_path),
                    "--out-dir",
                    str(media_dir / name),
                ],
                check=True,
            )
            generated += 1

    print(f"generated {generated} score/MIDI/WAV preview sets under {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
