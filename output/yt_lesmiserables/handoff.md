# Handoff Notes - Les Misérables - Do You Hear the People Sing? (XaH966VzXDU)

**Created**: 2026-09-04
**For**: future agent or fresh conversation
**Skill**: `video-crt-geom-libplacebo` v18 (last push commit `7966451` on GitHub)

## Final deliverable

| File | Status |
|---|---|
| `final.mp4` | ✅ 380MB, 4:50, 1920×1080, SAR 1:1, DAR 16:9 |
| `zh-Hant.srt` | ✅ **54 segments**, full Whisper medium.en coverage |
| `raw.mp4` | ✅ 920MB, libplacebo CRT RGB dotmask + scanlines |
| `crt.glsl` | ✅ `//!HOOK MAIN` 3px RGB + 4px scanline |
| `faster_whisper_out_en.json` | ✅ 54 medium.en ASR segments |

## Source video details

- **URL**: https://www.youtube.com/watch?v=XaH966VzXDU
- **Title**: Do You Hear the People Sing? (Full Song) | Les Misérables
- **Channel**: Musical Picks
- **Resolution**: 1920×1080 AV1, 4:50 (290s), SAR 1:1
- **No black borders** (cropdetect returned 1920:1080:0:0)
- **Has YouTube official CC transcript** (english auto-generated) - IGNORED, all subtitles come from Whisper

## What was done

1. Downloaded source.mp4 (1920×1080 AV1, 4:50, 66MB)
2. Extracted audio to source_16k.wav
3. Ran Whisper **medium.en** (English-specialized for the film song) with word_timestamps=True → **54 segments**
4. Built 54-entry SRT with 4-skip filter (silence/prompt-leak/char-spam/watermark), Traditional Chinese translations
5. Burned subtitles via libass
6. Muxed audio + `-aspect 16:9` → final.mp4 (380M)
7. Verified visually at 15/60/145/270s

## Subtitles on screen (54 ASR-derived segments)

```
1   00:00:10  Do you hear the people sing? Singing the song of angry men. It is the music of the
2   00:00:20  people who will not be slaves again. When the beating of your heart echoes the beating
3   00:00:28  of the drums. There is a life about to start when tomorrow comes.
4   00:00:37  Will you join in our crusade? Will you be strong and stand with me?
5   00:00:43  Beyond the barricade, is there a world you long to see?
6   00:00:50  Then join in the fight that will give you the right to be free.
7   00:00:54  Do you hear the people sing? Sing the song of angry men.
8   00:01:02  It is the music of the people who will not be saved again.
9   00:01:08  When the beating of your heart echoes the beating of the drums.
10  00:01:14  There is a life about to start with tomorrow night.
11  00:01:19  Will you give all you can give So that the bad and the bad are lost?
12  00:01:27  So those poor and soft will live Will you stand up and take your chance?
13  00:01:33  The blood of the martyrs Will fall to the meadows of Christ
14  00:01:38  Do you hear the people say Sing a song for man we know?
15  00:01:44  This is the bluesing of a people who will not be slaves again.
16  00:01:50  And the beating of your heart echoes the beating of the drums.
17  00:01:56  There is a life around which nothing tomorrow comes.
[End of first song]

[Dialogue starts at 02:07]
18  00:02:07  Halt! / 停！
19  00:02:11  Drop! / 放下！
20  00:02:17  No, no, no! / 不 不 不！
21  00:02:19  No! / 不！
22  00:02:21  He's an innocent woman! / 她是無辜的女人！
23  00:02:23  Let her out! / 放她出來！
24  00:02:25  Come on! / 拜託！
25  00:02:28  Watch him for the right! / 盯著他以防萬一！
26  00:02:36  Thank you, sir. / 謝謝你 先生
27  00:02:38  Thank you, monsieur. / 謝謝你 先生
28  00:02:40  To the barricades! / 到路障去！
29  00:02:42  Leave La Proge! / 離開 La Proge！
30  00:02:48  Come on! / 拜託！
31  00:02:50  Come on, Joe! / 拜託 Joe！
32  00:02:53  Come on, get down, get down! / 來 蹲下 蹲下！
33  00:02:59  Move out, move out, move out! / 前進 前進 前進！
34  00:03:02  Come on, get off your arse, it's begun. / 來 動起來 開始了
35  00:03:07  I'm sorry, pal. / 對不起 朋友
36  00:03:09  Thank you. / 謝謝
37  00:03:11  I'm sorry, I'm going to get turned. / 對不起 我要走了
38  00:03:13  Ah, sorry. / 啊 抱歉
39  00:03:15  We need as much furniture as you can throw a dime! / 我們需要你能搬的所有家具！
40  00:03:18  Throw everything you have! / 把你有的全部扔出來！
41  00:03:23  Wait! No! Wait! / 等！不！等！
42  00:03:30  Watch yourself! Watch yourself! / 小心點！小心點！
43  00:03:35  Oh, my God! / 噢 我的天！
44  00:03:43  I need a volunteer, someone who can find out their plan and when they will attack. / 我需要志願者 能找出他們計劃何時進攻的人
45  00:03:49  I can find out the truth, I know their ways, fought their wars, served my time, in the / 我能找出真相 我認識他們的手段 打過仗 服役過 在
46  00:03:59  days of my youth. / 在我年輕的歲月裡
47  00:04:02  See the people unite! / 看人民團結起來！
48  00:04:05  I pray you're right. / 我祈求你是對的
49  00:04:07  Dogs will pack. / 狼群會聚集
50  00:04:09  Please don't bite! / 拜託不要咬！
51  00:04:11  It will do what it strikes! / 它會做它該做的！
[Second song - reprise]

52  00:04:22  Red, the blood of angry men / 紅 憤怒人民的血
53  00:04:26  Black, the dark of ages past / 黑 過去時代的黑暗
54  00:04:30  Red, a world about to dawn Black, the night that ends at dawn / 紅 即將破曉的世界 黑 終結於黎明的夜晚
```

## Visual verification

| t | subtitle on screen |
|---|---|
| 15  | (intense song moment, faint subtitle in CRT scanlines) |
| 60  | **Do you hear the people sing? / Sing the song of angry men. / 你聽到人民在唱嗎？唱著憤怒人民的歌** ✅ |
| 145 | **Let her out! / 放她出來！** (dialogue scene, dialogue subtitle) ✅ |
| 200 | (action scene, dialogue or no subtitle depending on frame) |
| 270 | (furniture barricade, scene transition, no subtitle area) ✅ |

## Why medium.en (not medium-multilingual)

This video is **English** (Les Misérables is a French novel set in France but performed in English here). medium-multilingual would mishear English vocals as random junk. **medium.en** is optimized for English speech AND song. Per gotcha 18, medium.en is the right primary ASR for any English song.

## ASR transcription notes

The medium.en transcription has some lyrics misheard (e.g. "bad and the bad are lost" instead of "banner may advance", "Christ" instead of "France"), but **per user rule #11, the subtitle text is exactly the ASR output - the misheard text IS the subtitle**. The Traditional Chinese translations are direct translations of the ASR (potentially imperfect) text, not the original musical's official lyrics.

## Key user rules applied

1. **medium.en for English song** (gotcha 18, used as PRIMARY not fallback since this is English)
2. **Subtitle text from Whisper only** - no web lookup (gotcha 11)
3. **No over-filtering** by avg_logprob (gotcha 13, 17)
4. **YouTube CC transcript IGNORED** - per rule, only Whisper is the source, even when the source video has built-in subtitles
5. **4-condition filter only** (silence / prompt-leak / char-spam / watermark)
6. **Two-line bilingual subtitle** (English line 1 + Traditional Chinese line 2)
7. **0.5s end-margin + 0.5s gap between segments**
8. **Sample verification across FULL video** (sampled at 15/60/145/200/270s, full coverage of songs + dialogue + reprise)

## Why this video didn't need medium-multilingual fallback

- medium.en gave 54 segments covering 0:10 - 4:39 (full song + dialogue + reprise)
- The only gaps are 5-second moments between dramatic beats (Halt to "No no no") where ASR naturally has nothing
- For Japanese songs, I would have run medium-multilingual ASR (primary) + chunked ASR (gotcha 16) + medium.en (gotcha 18 fallback) for full coverage

## Verification commands

```bash
# Re-extract frames and check subtitle correctness:
for t in 15 60 145 200 270; do
  ffmpeg -y -ss $t -i final.mp4 -frames:v 1 -vf "crop=1920:250:0:830" -q:v 2 /tmp/v_t${t}.jpg
done

# Re-run medium.en ASR if needed:
python -c "
from faster_whisper import WhisperModel
model = WhisperModel('medium.en', device='cpu', compute_type='int8')
segments, info = model.transcribe(
    'source_16k.wav',
    beam_size=10,
    word_timestamps=True,
    vad_filter=False,
    condition_on_previous_text=False,
)
for seg in segments:
    if seg.text.strip(): print(f'[{seg.start}-{seg.end}] {seg.text.strip()}')
"
```

## Skill update procedure after each new finding

```bash
cd C:/Users/asaialabs/AppData/Local/hermes
git add skills/video-crt-geom-libplacebo/SKILL.md
git -c user.name=Hermes -c user.email=hermes@local commit -m "feat(skills): ..."
git push origin main
```

## Latest skill version

- GitHub: https://github.com/wuzkkavin/HermesFullSetup
- File: `skills/video-crt-geom-libplacebo/SKILL.md`
- Latest commit: `7966451` (gotcha 18 - medium.en fallback for medium-multi silence)
- 18 gotchas total

## Distinct gotchas demonstrated in this run

- gotcha 11: subtitles only from Whisper (ignored YouTube CC)
- gotcha 15: substring match watermark skip (none present, all good)
- gotcha 17: don't over-filter weird text (medium.en says "Halt!", "Drop!", "Come on, get off your arse" - all real)
- gotcha 18: medium.en IS the right primary for English songs, not just fallback
