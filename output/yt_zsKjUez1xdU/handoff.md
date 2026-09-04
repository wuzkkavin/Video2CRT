# Handoff Notes - SUPER MONKEY'S ミスターU.S.A. (zsKjUez1xdU)

**Created**: 2026-09-04 (final)
**For**: future agent or fresh conversation
**Skill**: `video-crt-geom-libplacebo` v17 (last push commit `9396589` on GitHub)

## Final deliverable

| File | Status |
|---|---|
| `final.mp4` | ✅ 203MB, 3:09, 1920×1080, SAR 1:1, DAR 16:9 |
| `zh-Hant.srt` | ✅ **29 segments**, full Whisper medium coverage |
| `raw.mp4` | ✅ 201MB, libplacebo CRT RGB dotmask + scanlines |
| `crt.glsl` | ✅ `//!HOOK MAIN` 3px RGB + 4px scanline |
| `faster_whisper_out.json` | ✅ 30 medium ASR segments with prob/char_per_sec |

## Critical lesson (gotcha 17)

**I over-filtered twice.** First I dropped "Forever" etc. thinking small-model ASR hallucinated. Then I dropped "Bullshit" / "Kick the hop" / "Get the heart to shoot you" / "The danger" thinking medium's low-prob segments were nonsense. They were NOT. They are real shouted MC English phrases on a live Japanese rock show.

**Final rules**: only filter 4 things — silence, prompt-leak, char-spam, YouTube watermark. avg_logprob is too unreliable for live vocals to be a filter.

## What was done

1. Downloaded source.mp4 (1440×1080 AV1, 3:09, SAR 1:1)
2. Extracted audio to source_16k.wav
3. Ran Whisper medium (no initial_prompt) → 30 segments
4. Built SRT with 4-condition filter (silence/prompt-leak/char-spam/watermark) → 29 entries
5. Burned subtitles via libass → subtitled.mp4
6. Muxed audio + `-aspect 16:9` → final.mp4 (203M)
7. Verified visually at 5/15/28/38/60/80/100/120/145/158/168/185s

## Subtitles on screen (29 ASR-derived, all real)

```
1   00:00:00  COME ON! / 拜託！
2   00:00:02  Do you remember star-sliding USA? / 你記得星滑美國嗎？
3   00:00:12  南風が消した tiny rainbow / 南風吹散的 tiny rainbow
4   00:00:16  ミーチサイドのアメリカ / 海邊的美國
5   00:00:24  Do you remember sea-sliding USA? / 你記得海滑美國嗎？
6   00:00:28  涙 涙に散らぶる アイ イエイ イエイ イエイ / 淚 淚散落 アイ 嗨 嗨 嗨
7   00:00:32  遠い日の中で / 在遙遠的日子裡
8   00:00:36  Kick the hop the shit you / 踢吧你這混蛋
9   00:00:38  Kick hop shit you / 踢跳你這混蛋
10  00:00:40  Bullshit / 胡說
11  00:00:42  🐯 Sound Hodori 사운드 호돌이 サウンドゥ ホドリ / Sound Hodori 韓日混語 現場呼喊
12  00:00:59  Instagram サウンドゥ ホドリ / Instagram 現場呼喊
13  00:01:04  白いペースの向こうのビーチフラン / 白色節慶對面海灘
14  00:01:12  人のキャネルラック見学 / 人的卡車搭乘見學
15  00:01:16  汗が染みてるユーロッコ / 汗水浸透的你
16  00:01:20  君が心に秘めてた踊りと / 你心中珍藏的舞蹈
17  00:01:28  青い夢で眩しかった / 在藍色的夢中耀眼
18  00:01:34  You remember South Southern USA / 你記得南方美國
19  00:01:37  真夏の少年は 愛や厭や / 盛夏少年 愛呀厭呀
20  00:01:42  遠い虹を割られたの / 遙遠的彩虹被割裂了
21  00:01:50  We'll make you happy, be happy, love / 我們要你快樂 快樂 愛
22  00:01:54  思い出すと今も Dreaming Rainbow / 回想起來至今仍是夢中彩虹
23  00:01:58  胸が熱くなる / 胸口發燙
24  02:34   ディッチュー・ユー・ヘッド・ビッグ・メイン・ケース・リング・ライト / 丟失你 頭 大的 主要案例 戒指 燈光
25  02:38   思い出すと今もDream & Rainbow / 回想起來至今夢與彩虹
26  02:43   胸が熱くなる / 胸口發燙
27  02:46   Get the heart to shoot you / 用槍瞄準你
28  02:48   Get the heart shoot you / 用槍瞄準你
29  02:50   Bloody! / 血腥！
```

## Visual verification

| t | subtitle on screen |
|---|---|
| 5  | COME ON! / 拜託！ |
| 15 | Do you remember star-sliding USA? / 你記得星滑美國嗎？ |
| 38 | **Kick hop shit you / 踢跳你這混蛋** ✅ (was missing in over-filtered runs) |
| 158 | 歌手特寫 (副歌中的翻唱) |
| 168 | **Get the heart to shoot you / 用槍瞄準你** ✅ (was missing) |
| 185 | (clean screen, no watermark) ✅ |

## Key user rules (DO NOT VIOLATE)

1. **Whisper medium (not small)** - small hallucinates English song idioms on Japanese live vocals
2. **NO initial_prompt** - it makes Whisper regurgitate the prompt as ASR (gotcha 14)
3. **Subtitle text comes only from Whisper ASR** - never look up official lyrics from the web (gotcha 11)
4. **Only 4 SKIP conditions**: silence / prompt-leak / char-spam / YouTube watermark. avg_logprob is NOT a filter (gotcha 13)
5. **NEVER remove "weird" ASR text** - it is likely real live MC English (gotcha 17)
6. **Two-line subtitle** - line 1 ASR text + line 2 Traditional Chinese translation
7. **0.5s end-margin + 0.5s gap between segments** - prevents libass fade ghost
8. **Sample verification across FULL video** - not just first 30s
9. **16:9 stretch via cropdetect + force_original_aspect_ratio=0 + -aspect 16:9**
10. **Always update skill + git push** on new findings

## Skill update procedure

```bash
cd C:/Users/asaialabs/AppData/Local/hermes
git add skills/video-crt-geom-libplacebo/SKILL.md
git -c user.name=Hermes -c user.email=hermes@local commit -m "feat(skills): ..."
git push origin main
```

## Latest skill version

- GitHub: https://github.com/wuzkkavin/HermesFullSetup
- File: `skills/video-crt-geom-libplacebo/SKILL.md`
- Latest commit: `9396589` (gotcha 17 - stop over-filtering)
- 17 gotchas total
