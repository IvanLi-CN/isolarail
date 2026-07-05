#!/usr/bin/env python3
"""Generate score JSON, MIDI, and WAV previews from the HTML tone data."""

from __future__ import annotations

import argparse
import html.parser
import json
import math
import shutil
import struct
import sys
import wave
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


def varlen(value: int) -> bytes:
    if value < 0:
        raise ValueError("MIDI delta cannot be negative")
    out = bytearray([value & 0x7F])
    value >>= 7
    while value:
        out.insert(0, 0x80 | (value & 0x7F))
        value >>= 7
    return bytes(out)


def midi_event(delta_ticks: int, payload: bytes) -> bytes:
    return varlen(delta_ticks) + payload


def freq_to_nearest_midi(freq_hz: float) -> int:
    if freq_hz <= 0:
        raise ValueError(f"freq_hz must be > 0, got {freq_hz}")
    return max(0, min(127, int(round(69 + 12 * math.log2(freq_hz / 440.0)))))


def write_midi(path: Path, score: dict) -> None:
    tempo_bpm = float(score["tempo_bpm"])
    ppqn = 480
    channel = int(score["midi"]["channel"])
    program = int(score["midi"]["program"])
    velocity = int(score["midi"]["velocity"])
    tempo_us_per_quarter = int(round(60_000_000 / tempo_bpm))

    track = bytearray()
    track.extend(midi_event(0, b"\xFF\x51\x03" + tempo_us_per_quarter.to_bytes(3, "big")))
    track.extend(midi_event(0, bytes([0xC0 | channel, program])))

    pending_delta = 0
    for event in score["events"]:
        duration_ms = int(event.get("ms") or event.get("rest_ms") or 0)
        ticks = int(round((duration_ms / 1000.0) * tempo_bpm * ppqn / 60.0))
        if "rest_ms" in event:
            pending_delta += max(0, ticks)
            continue

        midi_note = freq_to_nearest_midi(float(event["freq_hz"]))
        track.extend(midi_event(pending_delta, bytes([0x90 | channel, midi_note, velocity])))
        track.extend(midi_event(max(0, ticks), bytes([0x80 | channel, midi_note, 0])))
        pending_delta = 0

    track.extend(midi_event(pending_delta, b"\xFF\x2F\x00"))
    header = (
        b"MThd"
        + (6).to_bytes(4, "big")
        + (0).to_bytes(2, "big")
        + (1).to_bytes(2, "big")
        + ppqn.to_bytes(2, "big")
    )
    track_chunk = b"MTrk" + len(track).to_bytes(4, "big") + bytes(track)
    path.write_bytes(header + track_chunk)


def envelope(index: int, total: int, fade_samples: int) -> float:
    if fade_samples <= 0 or total <= 1:
        return 1.0
    start = index / fade_samples if index < fade_samples else 1.0
    end = (total - 1 - index) / fade_samples if index >= total - fade_samples else 1.0
    return max(0.0, min(1.0, start, end))


def write_wav(path: Path, score: dict) -> None:
    audio = score["audio"]
    sample_rate = int(audio["sample_rate_hz"])
    volume = max(0.0, min(1.0, float(audio["volume"]))) * 0.9
    fade_samples = max(0, int(round(sample_rate * int(audio["fade_ms"]) / 1000.0)))

    frames = bytearray()
    phase = 0.0
    for event in score["events"]:
        duration_ms = int(event.get("ms") or event.get("rest_ms") or 0)
        count = max(0, int(round(duration_ms * sample_rate / 1000.0)))
        if "rest_ms" in event:
            frames.extend(b"\x00\x00" * count)
            continue

        freq_hz = float(event["freq_hz"])
        increment = freq_hz / sample_rate
        for index in range(count):
            amp = envelope(index, count, fade_samples) * volume
            sample = amp if phase < 0.5 else -amp
            frames.extend(struct.pack("<h", int(sample * 32767)))
            phase += increment
            if phase >= 1.0:
                phase -= 1.0

    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(frames)


def prepare_out_dir(out_dir: Path, default_out_dir: Path) -> None:
    marker_name = ".buzzer-preview-generated"
    out_dir = out_dir.expanduser()
    default_out_dir = default_out_dir.expanduser()

    if out_dir.is_symlink():
        raise ValueError(f"refusing to use symlink output directory: {out_dir}")

    marker = out_dir / marker_name
    is_default_out_dir = out_dir.resolve(strict=False) == default_out_dir.resolve(strict=False)

    if out_dir.exists():
        if not out_dir.is_dir():
            raise ValueError(f"output path is not a directory: {out_dir}")
        if is_default_out_dir or marker.exists():
            shutil.rmtree(out_dir)
        elif any(out_dir.iterdir()):
            raise ValueError(
                f"refusing to delete non-generated output directory: {out_dir}; "
                f"use an empty directory or one containing {marker_name}"
            )

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / marker_name).write_text("generated by tools/buzzer_audio_preview/generate_assets.py\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    script_dir = Path(__file__).resolve().parent
    default_out_dir = script_dir / "out"
    parser.add_argument("--index", type=Path, default=script_dir / "index.html")
    parser.add_argument("--out-dir", type=Path, default=default_out_dir)
    args = parser.parse_args()

    data = load_tone_data(args.index)
    scores_dir = args.out_dir / "scores"
    media_dir = args.out_dir / "media"
    prepare_out_dir(args.out_dir, default_out_dir)
    scores_dir.mkdir(parents=True)
    media_dir.mkdir(parents=True)

    generated = 0
    for tone in data["tones"]:
        for candidate in tone["candidates"]:
            name = f"{slug(tone['id'])}__{slug(candidate['id'])}"
            score_path = scores_dir / f"{name}.json"
            score = score_for(data, candidate["events"])
            score_path.write_text(json.dumps(score, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            candidate_dir = media_dir / name
            candidate_dir.mkdir()
            write_midi(candidate_dir / f"{name}.mid", score)
            write_wav(candidate_dir / f"{name}.wav", score)
            generated += 1

    print(f"generated {generated} score/MIDI/WAV preview sets under {args.out_dir}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2)
