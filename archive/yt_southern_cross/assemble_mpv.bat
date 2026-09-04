@echo off
echo === Assemble final mp4 from mpv frames + audio + subtitles ===
cd C:\Users\asaialabs\Downloads\yt_southern_cross

REM Stage 1: PNG sequence -> lossless h264 mp4 (CPU encoding, fast preset)
ffmpeg -y -framerate 30 -i mpv_full_frames/%08d.png -i audio.m4a ^
  -c:v libx264 -preset fast -crf 18 -pix_fmt yuv420p ^
  -c:a copy -shortest ^
  temp_video.mp4
if errorlevel 1 goto :err

REM Stage 2: Burn in subtitles (fast nvenc encode, GPU)
ffmpeg -y -hwaccel cuda -i temp_video.mp4 ^
  -filter_complex "[0:v]scale=1920:1080:flags=lanczos,setsar=1,subtitles=zh-Hant_v3.srt:force_style='FontName=Microsoft JhengHei,FontSize=26,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2,Shadow=0,MarginV=10'[vout]" ^
  -map "[vout]" -map 0:a ^
  -c:v h264_nvenc -preset p4 -rc vbr -b:v 5M ^
  -c:a copy ^
  FINAL_mpv_crt-geom_d_RGB_dotmask_字幕.mp4
if errorlevel 1 goto :err

REM Cleanup
del temp_video.mp4
echo === DONE ===
goto :eof
:err
echo === FAILED ===