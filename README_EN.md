# Vibbon — Video Watermark Service (Rust + Actix + FFmpeg)

Vibbon is a lightweight **web service** that takes user-generated videos (UGC) and automatically applies a **transparent PNG frame/watermark** using **FFmpeg**. It’s designed for brand & event campaigns that need **consistent, vertical 9:16** outputs ready for TikTok, IG Reels, and YouTube Shorts.

> Repo: `https://github.com/kukuhtw/vibbon_rust`
> Demo video: `https://www.youtube.com/watch?v=ffhgjGxagnA`

---

## ✨ Features

* **Fast & efficient** pipeline powered by **Rust** + **Actix Web**
* **Automatic PNG overlay** (transparent frame/watermark)
* **Portrait 9:16 output** (default **720×1280**) optimized for social media
* **Quality controls** via **CRF** and **x264 preset**
* **Max duration enforcement** with optional auto-trim
* **Fill mode**: `crop` (scale-to-cover) or `pad` (letterbox)
* **Simple HTTP API** (multipart upload)

---

## 🧱 Tech Stack

* **Rust** (async with Tokio)
* **Actix Web** (HTTP server, multipart upload)
* **FFmpeg** (video processing)
* **once\_cell / Lazy** (binary path resolution)
* **sanitize-filename** (safe uploads)
* **uuid** (unique filenames)

---

## 📦 Requirements

* **Rust** 1.75+ (stable recommended)
* **FFmpeg** 4.2+ available on `PATH`

  * **Linux/macOS**: ensure `ffmpeg` is installed and discoverable
  * **Windows**: Vibbon will try `C:\ffmpeg\bin` automatically; or add FFmpeg to `PATH`

---

## 🚀 Quick Start

```bash
# 1) Clone
git clone https://github.com/kukuhtw/vibbon_rust.git
cd vibbon_rust

# 2) Build
cargo build --release

# 3) Run
./target/release/vibbon_rust
# Server starts (default 0.0.0.0:8080 unless configured in code)
```

Upload a video + overlay via `curl` (multipart):

```bash
curl -X POST http://localhost:8080/api/v1/process \
  -F "video=@samples/input.mp4" \
  -F "overlay=@overlays/frame.png" \
  -F "fill_mode=crop" \
  -F "crf=23" \
  -F "preset=veryfast" \
  -F "out_w=720" \
  -F "out_h=1280" \
  -F "allow_trim=1" \
  -F "max_duration_sec=30"
```

**Response (JSON)**

```json
{
  "ok": true,
  "output": "/outputs/vibbon-5a9c...c1.mp4",
  "width": 720,
  "height": 1280,
  "fill_mode": "crop",
  "crf": 23,
  "preset": "veryfast",
  "duration_sec": 12.34
}
```

The processed file is served as a static file (e.g., `/outputs/...mp4`).

> Tip: If you don’t send an `overlay`, the service can use a default overlay (e.g., `./overlays/frame.png`) if configured in code.

---

## ⚙️ Configuration

Most defaults are set as **constants** at the top of the code (see `main.rs`):

* `MAX_DURATION_SEC = 30.0`
* `OUT_WIDTH = 720`
* `OUT_HEIGHT = 1280`
* `CRF = 23`
* `PRESET = "veryfast"`
* `ALLOW_TRIM = true`
* `FILL_MODE = "crop"`  (`"crop"` or `"pad"`)

You can:

* Change these defaults in code, **or**
* Provide overrides per-request via form fields (`crf`, `preset`, `out_w`, `out_h`, `fill_mode`, `allow_trim`, `max_duration_sec`).

**FFmpeg path resolution**

* On Unix-like systems, the binary is resolved via `which ffmpeg`.
* On Windows, Vibbon additionally checks `C:\ffmpeg\bin`.

---

## 🧩 API

### `POST /api/v1/process` — Process a video

**Content-Type**: `multipart/form-data`

**Fields**

* `video` **(required)**: the input video file (mp4/mov/webm…)
* `overlay` *(optional)*: PNG with transparency to overlay
* `overlay_url` *(optional)*: URL to a PNG to fetch and use as overlay
* `out_w` *(int, default 720)*
* `out_h` *(int, default 1280)*
* `fill_mode` *(enum: `crop`|`pad`, default `crop`)*
* `crf` *(int, default 23; lower = higher quality, typically 18–28)*
* `preset` *(string, default `veryfast`; x264 presets: ultrafast…veryslow)*
* `allow_trim` *(0|1, default 1)*: if 1 and duration > `max_duration_sec`, auto-trim
* `max_duration_sec` *(float, default 30.0)*

**Success (200)**

```json
{
  "ok": true,
  "output": "/outputs/....mp4",
  "width": 720,
  "height": 1280,
  "duration_sec": 9.87
}
```

**Errors (4xx/5xx)**

```json
{ "ok": false, "error": "Missing 'video' file" }
```

---

## 🏗️ How It Works

1. **Upload**: Accept multipart (`video`, optional `overlay` or `overlay_url`).
2. **Validate & sanitize** filenames; save to `/uploads`.
3. **Inspect & (optionally) trim** if longer than `max_duration_sec` and `allow_trim=1`.
4. **Scale** to fit the requested output size (default 720×1280):

   * `crop` → scale-to-cover then center-crop (no letterbox)
   * `pad`  → scale-to-fit then add black bars (letterbox)
5. **Overlay** the transparent PNG frame on top.
6. **Encode** with H.264 (`libx264`, `CRF`, `preset`) and `aac` audio.
7. **Serve** the result from `/outputs` via static files.

**Representative FFmpeg filter (crop mode)**

```bash
ffmpeg -i input.mp4 -i frame.png \
  -filter_complex "[0:v]scale=720:1280:force_original_aspect_ratio=increase,\
crop=720:1280,setsar=1[v];[v][1]overlay=0:0" \
  -c:v libx264 -preset veryfast -crf 23 -c:a aac -movflags +faststart out.mp4
```

**Representative FFmpeg filter (pad mode)**

```bash
ffmpeg -i input.mp4 -i frame.png \
  -filter_complex "[0:v]scale=720:1280:force_original_aspect_ratio=decrease,\
pad=720:1280:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1[v];[v][1]overlay=0:0" \
  -c:v libx264 -preset veryfast -crf 23 -c:a aac -movflags +faststart out.mp4
```

---

## 📁 Suggested Project Structure

```
vibbon_rust/
├─ src/
│  └─ main.rs
├─ overlays/
│  └─ frame.png             # default overlay (optional)
├─ uploads/                 # temporary uploads (gitignored)
├─ outputs/                 # processed videos (gitignored)
├─ samples/
│  └─ input.mp4
├─ Cargo.toml
└─ README.md
```

> `uploads/` and `outputs/` are typically served statically by Actix (`actix_files::Files`).

---

## 🔒 Notes on Security & Limits

* Uploaded filenames are sanitized; unique names are generated.
* Max duration limit (default 30s) prevents abuse; adjust as needed.
* Consider reverse proxy limits (Nginx `client_max_body_size`, etc.).
* For production, place behind a WAF/proxy and enforce auth/rate limits if exposed publicly.

---

## 🧭 Roadmap

* [ ] Optional queue/offline processing
* [ ] Config via env (`.env`) instead of compile-time constants
* [ ] S3/GCS object storage adapters
* [ ] Positionable overlays (x/y, gravity)
* [ ] Multiple overlays / text layers
* [ ] Web UI dropzone

---

## 🤝 Contributing

PRs and issues are welcome! If you plan a larger change, please open an issue first to discuss scope and approach.

---

## 👤 Author & Contact

**Kukuh TW**

* Email: **[kukuhtw@gmail.com](mailto:kukuhtw@gmail.com)**
* WhatsApp: **[https://wa.me/628129893706](https://wa.me/628129893706)**
* LinkedIn: **[https://id.linkedin.com/in/kukuhtw](https://id.linkedin.com/in/kukuhtw)**
* X/Twitter: **@kukuhtw**
* Instagram: **@kukuhtw**
* Facebook: **[https://www.facebook.com/kukuhtw](https://www.facebook.com/kukuhtw)**

---

## 📜 License


