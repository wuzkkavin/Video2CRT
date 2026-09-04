"""ffmpeg pipeline runner - libplacebo shader + subtitle burn + mux."""
from pathlib import Path
import subprocess


LIBPLACEBO_SHADER = """\
//!HOOK MAIN
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);float x=HOOKED_pos.x*HOOKED_size.x;int col=int(mod(x,3.0));float strength=0.10;vec3 tint=vec3(0.0);if(col==0)tint=vec3(strength,-strength*0.3,-strength*0.3);else if(col==1)tint=vec3(-strength*0.3,strength,-strength*0.3);else tint=vec3(-strength*0.3,-strength*0.3,strength);float y=HOOKED_pos.y*HOOKED_size.y;int row=int(mod(y,4.0));float vfade=(row<2)?1.0:0.75;return vec4((c.rgb+tint)*vfade,c.a);}
"""


def render_raw(source: Path, raw_out: Path, *, crop: str = "", shader_path: Path | None = None) -> Path:
    """Stage 6 (WORKFLOW.md): render CRT shader via libplacebo.

    Args:
        source: source.mp4
        raw_out: raw.mp4 output path
        crop: optional crop filter like "1440:1080:240:0" (gotcha 6)
        shader_path: optional custom GLSL shader. If None, uses LIBPLACEBO_SHADER.
    """
    if shader_path is None:
        shader_path = raw_out.parent / "crt.glsl"
        shader_path.write_text(LIBPLACEBO_SHADER, encoding="utf-8")

    vf = []
    if crop:
        vf.append(f"crop={crop}")
    vf.append("libplacebo=custom_shader_path=" + str(shader_path) +
              ":w=1920:h=1080:fps=30:force_original_aspect_ratio=0")
    vf_str = ",".join(vf)

    r = subprocess.run(
        ["ffmpeg", "-y", "-hwaccel", "cuda", "-i", str(source),
         "-vf", vf_str,
         "-an", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "18",
         "-pix_fmt", "yuv420p", str(raw_out)],
        capture_output=True, text=True, timeout=900,
    )
    if r.returncode != 0:
        raise RuntimeError(f"libplacebo render failed: {r.stderr}")
    return raw_out


def burn_subtitles(raw: Path, srt: Path, subtitled: Path) -> Path:
    """Stage 8 (WORKFLOW.md): burn subtitles onto raw video."""
    r = subprocess.run(
        ["ffmpeg", "-y", "-i", str(raw),
         "-vf", f"subtitles={srt}:force_style='FontName=Microsoft JhengHei,FontSize=24,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2,Shadow=0,MarginV=10'",
         "-c:v", "h264_nvenc", "-preset", "p4", "-cq", "23", "-pix_fmt", "yuv420p",
         "-an", str(subtitled)],
        capture_output=True, text=True, timeout=300,
    )
    if r.returncode != 0:
        raise RuntimeError(f"subtitle burn failed: {r.stderr}")
    return subtitled


def mux_audio(subtitled: Path, source: Path, final: Path) -> Path:
    """Stage 9 (WORKFLOW.md): mux audio + -aspect 16:9 (gotcha 3)."""
    r = subprocess.run(
        ["ffmpeg", "-y", "-i", str(subtitled), "-i", str(source),
         "-map", "0:v", "-map", "1:a",
         "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
         "-aspect", "16:9", "-shortest", str(final)],
        capture_output=True, text=True, timeout=120,
    )
    if r.returncode != 0:
        raise RuntimeError(f"mux failed: {r.stderr}")
    return final
