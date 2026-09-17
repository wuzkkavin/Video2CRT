"""Regression tests for the actual sidecar subtitle contract (no model/network)."""
import importlib.util
import sys
import types
import unittest
import json
import tempfile
from unittest.mock import patch
from pathlib import Path

APP = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APP / "src-tauri" / "bin"))
import pipeline_cli as pipeline
from subtitle_engine import (
    align_caption_translations,
    captions_match_language,
    clip_captions_to_duration,
    load_youtube_captions,
    merge_parallel_caption_tracks,
    build_srt_original,
    normalize_segments,
    select_source_captions,
    split_spoken_captions,
    traditional,
)


def blocks(srt):
    return [b.splitlines() for b in srt.strip().split("\n\n") if b.strip()]


class SubtitleContract(unittest.TestCase):
    def test_whisper_word_alignment_index_error_retries_without_word_timestamps(self):
        """A faster-whisper alignment bug must not abort the whole video."""
        spec = importlib.util.spec_from_file_location(
            "asr_retry_test", APP.parent / "src" / "video2crt" / "asr.py")
        asr = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(asr)

        calls = []
        class FakeModel:
            def __init__(self, *_args, **_kwargs):
                pass
            def transcribe(self, _audio, **kwargs):
                calls.append(kwargs["word_timestamps"])
                info = types.SimpleNamespace(language="ja", language_probability=0.99)
                def segments():
                    if kwargs["word_timestamps"]:
                        raise IndexError("boolean index did not match indexed array")
                    yield types.SimpleNamespace(
                        start=0.0, end=2.0, text="テスト", words=[],
                        avg_logprob=-0.1, no_speech_prob=0.0)
                return segments(), info

        with patch.dict(sys.modules, {"faster_whisper": types.SimpleNamespace(WhisperModel=FakeModel)}):
            result = asr.transcribe(Path("audio.wav"), model_name="local-model")
        self.assertEqual(calls, [True, False])
        self.assertEqual(result[0]["text"], "テスト")

    def test_subtitle_burn_keeps_approved_crt_encoder(self):
        """Subtitle work must retain the user-approved CRT output encoder."""
        captured = []
        with patch.object(pipeline.subprocess, "run", side_effect=lambda command, **_kwargs:
                          captured.append(command) or types.SimpleNamespace(returncode=0, stderr="")):
            pipeline.burn_subtitles_local(
                Path("raw.mp4"), Path("zh-Hant.srt"), Path("subtitled.mp4"), Path("."))
        command = captured[0]
        self.assertEqual(command[command.index("-c:v") + 1], "h264_nvenc")
        self.assertEqual(command[command.index("-cq") + 1], "23")

    def test_official_traditional_track_aligns_by_largest_time_overlap(self):
        source = [
            {"start": 9.0, "end": 15.2, "text": "明け行く夕日の中を今夜も昼下がり", "language": "ja"},
            {"start": 15.5, "end": 16.7, "text": "さらり", "language": "ja"},
        ]
        traditional_track = [
            {"start": 8.9, "end": 12.2, "text": "在逐漸破曉的夕陽餘暉", "language": "zh"},
            {"start": 12.3, "end": 17.1, "text": "今晚也轉為過午時分，毫無停滯", "language": "zh"},
        ]
        aligned = align_caption_translations(source, traditional_track)
        self.assertEqual(aligned[source[0]["text"]], "在逐漸破曉的夕陽餘暉")
        self.assertEqual(aligned[source[1]["text"]], "今晚也轉為過午時分，毫無停滯")

    def test_different_official_track_breaks_merge_into_complete_bilingual_cues(self):
        source = [
            {"start": 9.0, "end": 15.2, "text": "明け行く夕日の中を今夜も昼下がり", "language": "ja"},
            {"start": 15.5, "end": 16.7, "text": "さらり", "language": "ja"},
            {"start": 16.9, "end": 23.0, "text": "どれほど朽ち果てようと", "language": "ja"},
        ]
        traditional_track = [
            {"start": 8.9, "end": 12.2, "text": "在逐漸破曉的夕陽餘暉", "language": "zh"},
            {"start": 12.3, "end": 17.1, "text": "今晚也轉為過午時分，毫無停滯", "language": "zh"},
            {"start": 17.2, "end": 20.4, "text": "即便再怎麼腐朽消逝", "language": "zh"},
        ]
        merged, translations = merge_parallel_caption_tracks(source, traditional_track)
        self.assertEqual(merged[0]["text"], "明け行く夕日の中を今夜も昼下がり さらり")
        self.assertEqual(translations[merged[0]["text"]],
                         "在逐漸破曉的夕陽餘暉 今晚也轉為過午時分，毫無停滯")
        self.assertEqual(merged[1]["text"], "どれほど朽ち果てようと")

    def test_full_youtube_track_is_clipped_to_short_source_video(self):
        captions = [
            {"start": 28.0, "end": 31.0, "text": "Last visible", "language": "en"},
            {"start": 31.0, "end": 34.0, "text": "Outside", "language": "en"},
        ]
        clipped = clip_captions_to_duration(captions, 30.0)
        self.assertEqual(len(clipped), 1)
        self.assertEqual(clipped[0]["end"], 30.0)

    def test_youtube_caption_parser_keeps_lyrics_and_drops_sound_labels(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "source.en.vtt"
            path.write_text("""WEBVTT

00:00:00.600 --> 00:00:06.600
♪♪♪

00:00:06.600 --> 00:00:10.400
♪ I SEE TREES OF GREEN ♪

00:00:14.880 --> 00:00:17.960
♪ I SEE THEM BLOOM ♪

00:00:23.960 --> 00:00:24.360 align:start position:19%
[AUDIENCE APPLAUDS]
""", encoding="utf-8")
            captions = load_youtube_captions(path, "en")
        self.assertEqual([c["text"] for c in captions],
                         ["I SEE TREES OF GREEN", "I SEE THEM BLOOM"])
        self.assertEqual(captions[0]["start"], 6.6)
        self.assertEqual(captions[-1]["end"], 17.96)

    def test_youtube_caption_language_must_match_source_speech(self):
        self.assertTrue(captions_match_language(
            [{"text": "荒れ狂う季節の中を", "language": "ja"}], "ja"))
        self.assertFalse(captions_match_language(
            [{"text": "The translated lyric", "language": "en"}], "ja"))

    def test_low_confidence_wrong_asr_language_uses_youtube_language_metadata(self):
        selected = pipeline.prefer_declared_caption_language(
            [{"language_probability": 0.53, "text": "Субтитры делал DimaTorzok"}],
            "ru", "ja")
        self.assertEqual(selected, "ja")

    def test_strong_asr_language_is_not_replaced_by_upload_metadata(self):
        selected = pipeline.prefer_declared_caption_language(
            [{"language_probability": 0.95, "text": "Hello"}], "en", "ja")
        self.assertEqual(selected, "en")

    def test_complete_exact_language_caption_replaces_asr_mistake(self):
        asr = [
            {"start": 6.6, "end": 10.4, "text": "I see trees of green", "language": "en"},
            {"start": 14.8, "end": 18.0, "text": "I see them blue", "language": "en"},
        ]
        youtube = [
            {"start": 6.6, "end": 10.4, "text": "I SEE TREES OF GREEN", "language": "en"},
            {"start": 14.88, "end": 17.96, "text": "I SEE THEM BLOOM", "language": "en"},
        ]
        selected = select_source_captions(asr, youtube, "en")
        self.assertEqual(selected[1]["text"], "I SEE THEM BLOOM")

    def test_partial_youtube_caption_does_not_erase_complete_asr(self):
        asr = [
            {"start": 0, "end": 4, "text": "First", "language": "en"},
            {"start": 20, "end": 25, "text": "Last", "language": "en"},
        ]
        youtube = [{"start": 0, "end": 2, "text": "First", "language": "en"}]
        self.assertIs(select_source_captions(asr, youtube, "en"), asr)

    def test_traditional_phrase_conversion(self):
        self.assertEqual(traditional("什么？皇后在后面。"), "什麼？皇后在後面。")

    def test_long_speech_is_split_at_real_word_times(self):
        words = [{"start": i, "end": i + 0.8, "word": f" word{i}"} for i in range(18)]
        segments = split_spoken_captions([{"start": 0, "end": 18, "language": "en",
            "text": "".join(w["word"] for w in words).strip(), "words": words}])
        self.assertGreater(len(segments), 1)
        self.assertEqual(" ".join(s["text"] for s in segments), " ".join(w["word"].strip() for w in words))
        self.assertTrue(all(s["end"] - s["start"] <= 7 for s in segments))
    def test_chinese_is_single_traditional_line(self):
        result = pipeline.build_srt_two_line(
            [{"start": 0, "end": 2, "text": "我们一起学习。", "language": "zh"}],
            {"我们一起学习。": "我們一起學習。"}, emit_empty_translation=True)
        self.assertEqual(blocks(result)[0][2:], ["我們一起學習。"])

    def test_original_only_mode_never_adds_translation_line(self):
        result = build_srt_original([
            {"start": 0, "end": 2, "text": "Hello everyone.", "language": "en"}
        ])
        self.assertEqual(blocks(result)[0][2:], ["Hello everyone."])

    def test_first_caption_starts_with_voice(self):
        result = pipeline.build_srt_two_line(
            [{"start": 0, "end": 2, "text": "Hello everyone.", "language": "en"}],
            {"Hello everyone.": "大家好。"})
        self.assertEqual(blocks(result)[0][1], "00:00:00,000 --> 00:00:02,000")

    def test_fast_dialogue_does_not_overlap(self):
        result = pipeline.build_srt_two_line([
            {"start": 0, "end": 0.6, "text": "Yes.", "language": "en"},
            {"start": 0.6, "end": 1.1, "text": "No.", "language": "en"}],
            {"Yes.": "是。", "No.": "否。"})
        b = blocks(result)
        self.assertLessEqual(b[0][1].split(" --> ")[1], b[1][1].split(" --> ")[0])

    def test_same_start_asr_alternatives_do_not_abort_subtitles(self):
        result = normalize_segments([
            {"start": 10.0, "end": 10.3, "text": "short", "language": "en"},
            {"start": 10.0, "end": 12.0, "text": "complete spoken line", "language": "en"},
            {"start": 11.8, "end": 13.0, "text": "next line", "language": "en"},
        ])
        self.assertEqual([segment["text"] for segment in result],
                         ["complete spoken line", "next line"])
        self.assertLessEqual(result[0]["end"], result[1]["start"])

    def test_bad_timestamp_never_emitted(self):
        with self.assertRaisesRegex(ValueError, "時間無效"):
            pipeline.build_srt_two_line([
                {"start": 120.24, "end": 119.98, "text": "Hello.", "language": "en"}],
                {"Hello.": "你好。"})

    def test_missing_translation_cannot_silently_drop_speech(self):
        with self.assertRaisesRegex(ValueError, "translation|翻譯"):
            pipeline.build_srt_two_line([
                {"start": 0, "end": 2, "text": "Unseen sentence.", "language": "en"}], {})

    def test_japanese_kanji_is_not_treated_as_chinese(self):
        result = pipeline.build_srt_two_line([
            {"start": 0, "end": 2, "text": "今日", "language": "ja"}], {"今日": "今天"})
        self.assertEqual(blocks(result)[0][2:], ["今日", "今天"])

    def test_spoken_subscribe_is_not_deleted_as_watermark(self):
        result = pipeline.build_srt_two_line([
            {"start": 0, "end": 2, "text": "Please subscribe.", "language": "en"}],
            {"Please subscribe.": "請訂閱。"})
        self.assertEqual(len(blocks(result)), 1)

    def prepared_job(self, language="en", text="Hello everyone."):
        root = APP / "test-out" / "subtitle-contract"
        root.mkdir(parents=True, exist_ok=True)
        folder = Path(tempfile.mkdtemp(dir=root))
        (folder / "subtitle_segments.json").write_text(json.dumps([
            {"start": 0, "end": 2, "text": text, "language": language}]), encoding="utf-8")
        return folder

    def test_final_video_receives_translated_srt_before_burn(self):
        folder = self.prepared_job()
        (folder / "subtitle_translations.json").write_text(
            json.dumps({"Hello everyone.": "大家好。"}), encoding="utf-8")
        burned = []
        def burn(raw, srt, output, cwd):
            burned.append(srt.read_text(encoding="utf-8"))
        with patch.object(pipeline, "burn_subtitles_local", side_effect=burn), \
             patch.object(pipeline, "mux_audio_local") as mux:
            pipeline.run({"outputDir": str(folder), "phase": "finalize", "cloudTranslation": True})
        self.assertEqual(blocks(burned[0])[0][2:], ["Hello everyone.", "大家好。"])
        self.assertEqual(mux.call_args.args[0].name, "subtitled.mp4")

    def test_existing_verified_translation_skips_local_model(self):
        folder = self.prepared_job("ja", "さらり")
        (folder / "subtitle_translations.json").write_text(
            json.dumps({"さらり": "乾脆爽快"}), encoding="utf-8")
        with patch.object(pipeline, "translate_locally", side_effect=AssertionError("must not call")), \
             patch.object(pipeline, "burn_subtitles_local"), patch.object(pipeline, "mux_audio_local"):
            pipeline.run({"outputDir": str(folder), "phase": "finalize", "cloudTranslation": False})
        self.assertEqual(blocks((folder / "zh-Hant.srt").read_text(encoding="utf-8"))[0][2:],
                         ["さらり", "乾脆爽快"])

    def test_cloud_failure_never_burns_incomplete_video(self):
        folder = self.prepared_job()
        with patch.object(pipeline, "burn_subtitles_local") as burn, \
             patch.object(pipeline, "emit_done") as done:
            with self.assertRaises(ValueError):
                pipeline.run({"outputDir": str(folder), "phase": "finalize",
                              "cloudTranslation": True, "translationMode": "cloud"})
        burn.assert_not_called()
        done.assert_not_called()

    def test_chinese_needs_no_translation_model_or_key(self):
        folder = self.prepared_job("zh", "我们一起学习。")
        with patch.object(pipeline, "translate_locally", side_effect=AssertionError("must not call")), \
             patch.object(pipeline, "burn_subtitles_local"), patch.object(pipeline, "mux_audio_local"):
            pipeline.run({"outputDir": str(folder), "phase": "finalize", "cloudTranslation": True})
        self.assertEqual(blocks((folder / "zh-Hant.srt").read_text(encoding="utf-8"))[0][2:], ["我們一起學習。"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
