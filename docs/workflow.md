# Video2CRT - WORKFLOW

把一支 YouTube 影片從 URL 變成燒上雙語字幕 + CRT 效果的 1920×1080 MP4 的完整工作流程。

---

## Stage 1 — 下載

```bash
yt-dlp -P "C:/Users/asaialabs/Documents/Hermes/Video2CRT/" \
       -f "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best" \
       --merge-output-format mp4 \
       -o "%(title)s [%(id)s].%(ext)s" \
       "https://www.youtube.com/watch?v=XXXXX"
```

下載到當前目錄後**重新命名**成 `source.mp4` 並把整個 video-id 資料匣標準化：

```bash
mv "*[XXXXXXXXXXX].mp4" "source.mp4"
```

**產物**: `source.mp4` (原始 1920×1080, 帶側邊黑邊也沒關係)

---

## Stage 2 — 屬性檢查

```bash
# 解析度、編碼、時長
ffprobe -v error -show_entries stream=codec_name,width,height \
        -show_entries format=duration -of default source.mp4

# 黑邊偵測 (cropdetect)
ffmpeg -i source.mp4 -vf "cropdetect=24:2:0" \
       -frames:v 200 -f null - 2>&1 | grep -oE "crop=[0-9]+:[0-9]+:[0-9]+:[0-9]+" | tail -1
```

**決策**:
- 解析度 == 1920×1080, SAR 1:1, 無黑邊 → 直接 Stage 4
- 有側邊黑邊（4:3 內容塞 16:9 容器）→ 用 cropdetect 的 crop 參數

---

## Stage 3 — 音訊抽取（給 Whisper 用）

```bash
ffmpeg -y -i source.mp4 -vn -ar 16000 -ac 1 -c:a pcm_s16le source_16k.wav
```

**16kHz mono PCM** 是 faster-whisper 最佳輸入格式。

---

## Stage 4 — Whisper ASR

### Primary: medium-multilingual

```python
from faster_whisper import WhisperModel
model = WhisperModel("medium", device="cpu", compute_type="int8")

segments, info = model.transcribe(
    "source_16k.wav",
    language="ja",  # 或 "en" 看影片
    beam_size=10,
    word_timestamps=True,    # gotcha 13/18 需要 avg_logprob
    vad_filter=False,
    condition_on_previous_text=False,
    # NO initial_prompt — gotcha 14 會導致 prompt leakage
)
```

### Fallback 1: chunked ASR (gotcha 16)

```bash
# 把音訊切成 40 秒一段，每段獨立跑 medium
for t in 0 40 80 120 160; do
  ffmpeg -y -i source.mp4 -ss $t -t 40 -vn -ar 16000 -ac 1 clip_${t}.wav
done
# 對每段 clip_NNN.wav 跑 Whisper，把秒數校正回絕對時間
```

### Fallback 2: medium.en (gotcha 18)

對 medium-multilingual 沉默 30+ 秒段（例如日文 live 演唱會 122-152s 的歌手真有唱但 multi 沒輸出）：

```python
model = WhisperModel("medium.en", device="cpu", compute_type="int8")
segments, _ = model.transcribe("source_16k.wav",  # 沒帶 language
                                beam_size=10, word_timestamps=True,
                                vad_filter=False, condition_on_previous_text=False)
```

medium.en 對日文 live 會 romanize 成英文但**比沉默好**。

### Fallback 3: YouTube CC (gotcha 19-20)

```bash
# 抽 YouTube 自動 CC 字幕
yt-dlp -P "<path>" --write-auto-subs --convert-subs srt \
       -o "source.%(ext)s" \
       "https://www.youtube.com/watch?v=XXXXX"
# 會產生 *.en.vtt / *.ja.vtt 等
# 跟 Whisper 比對，per 5s slice 選最乾淨的（gotcha 20）
```

**禁止**的：j-lyric.net, animesonglyrics.com, genius.com 等外部歌詞站。

---

## Stage 5 — GLSL Shader

預設放在每支影片的 `crt.glsl`，內容：

```glsl
//!HOOK MAIN       ← gotcha 1: 不能用 //!HOOK RGB
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);float x=HOOKED_pos.x*HOOKED_size.x;int col=int(mod(x,3.0));float strength=0.10;vec3 tint=vec3(0.0);if(col==0)tint=vec3(strength,-strength*0.3,-strength*0.3);else if(col==1)tint=vec3(-strength*0.3,strength,-strength*0.3);else tint=vec3(-strength*0.3,-strength*0.3,strength);float y=HOOKED_pos.y*HOOKED_size.y;int row=int(mod(y,4.0));float vfade=(row<2)?1.0:0.75;return vec4((c.rgb+tint)*vfade,c.a);}
```

- `mod(x, 3.0)` = 3 px RGB dotmask
- `mod(y, 4.0)` = 4 px scanline
- `strength 0.10` = dotmask 強度
- `0.75` = scanline 暗度

微調：6/9 px dotmask 更細，0.05 強度更 subtile。

---

## Stage 6 — Raw video encode (libplacebo shader)

```bash
ffmpeg -y -hwaccel cuda -c:v av1_cuvid -i source.mp4 \
  -vf "crop=<H黑邊剪裁>,\
libplacebo=custom_shader_path=crt.glsl:w=1920:h=1080:fps=30:force_original_aspect_ratio=0" \
  -an -c:v libx264 -preset ultrafast -crf 18 -pix_fmt yuv420p raw.mp4
```

**Gotchas**:
- `force_original_aspect_ratio=0` (gotcha 2) — 不加會裁 16:9 的 4:3
- `-c:v av1_cuvid` (gotcha 5) — AV1 來源用，其他解碼用對應 cuvid
- `-pix_fmt yuv420p` — 必需

**產物**: `raw.mp4` (200-900MB depending on source)

---

## Stage 7 — Build SRT

從 `faster_whisper_out.json` 翻譯每段到繁體中文，產出 `zh-Hant.srt`。

```python
def fmt(t):
    h=int(t//3600); m=int((t%3600)//60); s=t-h*3600-m*60
    return f"{h:02d}:{m:02d}:{s:06.3f}".replace(".", ",")

srt = []
prev = 0
i = 0
for s in segs:
    text = s["text"]
    zh = trans.get(text)  # 你寫的字典
    if not zh: continue
    i += 1
    ns = max(s["start"] + 0.3, prev + 0.5)  # gotcha 9: 0.3-0.5s gap
    ne = max(s["end"] - 0.5, ns + 1.5)
    if ne <= ns: ne = ns + 2.0
    prev = ne
    srt += [f"{i}", f"{fmt(ns)} --> {fmt(ne)}", text, zh, ""]
```

**Gotchas**:
- 兩行 (gotcha 8) — 原文 + 繁中，**不加第三行**
- 0.3-0.5s gap (gotcha 9) — 避免 libass fade ghost
- 4-condition filter (gotcha 13) — silence / prompt-leak / char-spam / YT watermark
- **不要**用 avg_logprob filter 砍段 (gotcha 13, 17)
- Never web lyrics (gotcha 11)

---

## Stage 8 — Burn subtitles

```bash
ffmpeg -y -i raw.mp4 \
  -vf "subtitles=zh-Hant.srt:force_style='FontName=Microsoft JhengHei,FontSize=24,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2,Shadow=0,MarginV=10'" \
  -c:v h264_nvenc -preset p4 -cq 23 -pix_fmt yuv420p -an subtitled.mp4
```

**Gotchas**:
- Microsoft JhengHei (繁中)
- FontSize 24, white, 2 px black outline
- MarginV=10 (距底 10 px)

---

## Stage 9 — Mux + fix SAR (gotcha 3)

```bash
ffmpeg -y -i subtitled.mp4 -i source.mp4 \
  -map 0:v -map 1:a \
  -c:v copy -c:a aac -b:a 192k \
  -aspect 16:9 -shortest \
  final.mp4
```

**Gotcha 3**: 不加 `-aspect 16:9` 會讓播放器 letterbox 回 4:3。

---

## Stage 10 — Verify (gotcha 10)

抽多時間點截圖確認：

```bash
for t in 5 30 60 90 120; do
  ffmpeg -y -ss $t -i final.mp4 -frames:v 1 \
         -vf "crop=1200:200:360:850" -q:v 2 /tmp/v_t${t}.jpg
done
# 視覺檢查字幕出現位置、CRT 效果、沒有「請訂閱本頻道」浮水印
```

**Gotcha 10**: **跨整段影片**抽樣，不是只查前 30s。
**Gotcha 17**: 字幕在螢幕上看到 weird 文字是正常的（Live MC English）。

---

## Stage 11 — handoff.md

把整個流程記下供未來參考：

```markdown
# Handoff Notes - <Video Title> (<YouTube ID>)

## Source
- URL: https://www.youtube.com/watch?v=XXXXX
- Title: ...
- Resolution: 1920x1080 AV1, 4:50, SAR 1:1

## Final deliverable
| File | Size | Notes |
| final.mp4 | 380MB | 4:50 |
| zh-Hant.srt | 54 segments | Whisper medium.en |

## Subtitles (ASR-derived)
<list>

## Edge cases / lessons
<新發現>
```

---

## Stage 12 — Update project files

1. **CHANGELOG.md**: 加一行
2. **INDEX.md**: 更新影片列表
3. **skill** (if new gotcha): 加 gotcha → commit → push 到 GitHub

---

## 常見錯誤排除

### 「字幕顯示『請訂閱本頻道』」
→ gotcha 15 過濾。檢查 SRT 最後 10-15 秒。

### 「字幕在某段後空白 30s」
→ gotcha 18。跑 `medium.en` fallback。

### 「字幕變得稀少（比歌曲少）」
→ gotcha 17。**停止**用 avg_logprob 砍段。

### 「字幕有『日本語 ロックライブ』之類 prompt 殘留」
→ gotcha 14。**不要**用 `initial_prompt=`.

### 「播放器把 1920x1080 顯示成 4:3」
→ gotcha 3。mux step 加 `-aspect 16:9`.

### 「CRT 效果沒出來」
→ gotcha 1。確認 shader 用 `//!HOOK MAIN` 不是 `//!HOOK RGB`. 確認 `.glsl` 副檔名 (不是 `.hook`).

### 「輸出有綠色 tint」
→ 這個是 blend=multiply 在 YUV420p 引起的，libplacebo 在 RGB domain 不會有。如果出現 → 確認 shader 是 RGB domain。

---

最後更新: 2026-09-04
