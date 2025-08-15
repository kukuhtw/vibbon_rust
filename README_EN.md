# Vibbon — Video Watermark Solution for Brand & Event Campaigns

**Vibbon** is a lightweight **web service** built with **Rust + Actix** that processes **UGC videos** (user-generated content) and automatically applies a **transparent PNG frame / watermark** using **FFmpeg**. Perfect for **brand campaigns**, **event organizers**, and **UGC activations** on TikTok / IG Reels / YouTube Shorts.

> “A *video* version of a Twibbon” — more engaging, flexible duration, social-ready.
>
> Demo [https://youtu.be/ffhgjGxagnA](https://youtu.be/ffhgjGxagnA)

---

## ✨ Key Features

* **Transparent PNG Overlay** — apply frames/logos/watermarks to user videos.
* **Portrait, Social-Ready Output** — default **720×1280 (9:16)**.
* **Efficient Compression** — control **CRF** and **FFmpeg preset** (performance-friendly defaults).
* **Duration Limit** — auto-trim to a maximum duration (e.g., 30 seconds) for lightweight outputs.
* **Simple UI + API** — upload via web page or send over HTTP `multipart/form-data`.
* **Cross-OS** — Windows, Linux, macOS (requires FFmpeg in PATH).
* **Safe Filenames** — sanitize uploaded filenames to prevent harmful characters.

---

## 🏗️ Architecture Overview

* **Web server**: \[Actix Web] + \[actix-multipart] for uploads
* **Async runtime**: Tokio
* **Media engine**: FFmpeg (spawned as an external process)
* **Template**: HTML in `templates/` (simple front-end)
* **Utilities**: `once_cell`, `uuid`, `sanitize-filename`, `futures-util`, `which`

> On Windows, Vibbon automatically appends `C:\ffmpeg\bin` to the process `PATH` if that folder exists — making it easy to run `ffmpeg.exe` without manual PATH setup.

---

## 🧰 Requirements

* **Rust** (stable)
* **FFmpeg**:

  * **Windows**: extract to `C:\ffmpeg\bin` (or ensure `ffmpeg.exe` is in PATH).
  * **Ubuntu/Debian**: `sudo apt-get install ffmpeg`
  * **macOS**: `brew install ffmpeg`

---

## 🚀 Getting Started (Development)

```bash
# 1) Clone
git clone https://github.com/kukuhtw/vibbon_rust.git
cd vibbon_rust

# 2) Run (dev)
cargo run

# or build release
cargo build --release
./target/release/vibbon_rust
```

By default the server runs at `http://localhost:8080` (adjust as needed).

---

## 🌐 How to Use

### 1) Via Web Page

1. Open `http://localhost:8080`
2. Upload a video (MP4/MOV/WEBM) — ideally < 30 seconds.
3. Choose a **PNG frame template** if available (or use the default).
4. Click **Render** → download the result (MP4) and share on social media.

### 2) Via API (HTTP Multipart)

Example `curl` (sample endpoint; match your routes in code):

```bash
curl -X POST http://localhost:8080/api/render \
  -F "video=@/path/to/video.mp4" \
  -F "overlay=@/path/to/frame.png" \
  -o output.mp4
```

**Common form fields:**

* `video` — input video file (required)
* `overlay` — transparent PNG file (optional if a default exists)
* (Optional, depending on implementation) `layout=crop|pad`, `max_duration=30`, `width=720`, `height=1280`, `crf=23`, `preset=veryfast`.

> Exact endpoint names & parameters: please check `src/` (Actix handlers/controllers). This README provides a general scheme to ease adoption.

---

## ⚙️ Configuration

Common constants you’ll find in the code (tune at source):

* `MAX_DURATION_SEC` — output duration cap (e.g., `30.0`)
* `OUT_WIDTH`, `OUT_HEIGHT` — output resolution (e.g., `720×1280`)
* `CRF`, `PRESET` — FFmpeg quality & speed
* `ALLOW_TRIM` — auto-trim when input exceeds the cap

> Tip: you can expose these as **env vars** in the future, or provide **per-request query/form params**.

---

## 🗂️ Directory Structure (brief)

```
vibbon_rust/
├─ src/                # Rust code (Actix, handlers, ffmpeg runner)
├─ templates/          # Simple HTML (upload form, etc.)
├─ Cargo.toml
└─ README.md
```

---

## 🧪 FFmpeg Flow Example (conceptual)

Basic overlay filter (illustration; may differ in code):

```bash
ffmpeg -i input.mp4 -i overlay.png \
  -filter_complex "[0:v]scale=720:1280:force_original_aspect_ratio=increase,crop=720:1280[bg];[bg][1:v]overlay=(W-w)/2:(H-h)/2" \
  -c:v libx264 -crf 23 -preset veryfast -c:a copy output.mp4
```

* **Crop mode**: scale-to-cover then crop to 9:16.
* **Pad mode**: scale-to-fit then letterbox (use the `pad` filter).

---

## 🔒 Security & Limits

* **Filenames** are sanitized (avoid dangerous characters).
* **File size**: enforce at your reverse proxy/web server as needed.
* **Duration**: use `MAX_DURATION_SEC` to prevent heavy jobs.
* **Temporary files**: ensure the `uploads/` (or similar) dir is writable and cleaned periodically.

---

## 🐳 (Optional) Run with Docker

```Dockerfile
FROM rust:1.79-slim AS build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ffmpeg ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/vibbon_rust /usr/local/bin/vibbon
EXPOSE 8080
CMD ["vibbon"]
```

```bash
docker build -t vibbon .
docker run --rm -p 8080:8080 vibbon
```

---

## 🧭 Roadmap

* [ ] Full parameterization via **ENV** (port, resolution, crf, preset, max duration)
* [ ] **Multi-template** support (choose frames from a list)
* [ ] Dynamic **text overlay** (campaign title, participant name)
* [ ] **Queue/worker** for job batching
* [ ] **Docker Compose** + reverse proxy
* [ ] **Unit tests** for the ffmpeg command builder

---

## 🤝 Contributing

Contributions welcome!
Please open an **issue** for bugs/feature requests, or a **pull request** if you’re ready to propose changes.

---

## 👤 Author

**Kukuh TW**
📧 Email: **[kukuhtw@gmail.com](mailto:kukuhtw@gmail.com)**
📱 WhatsApp: **[https://wa.me/628129893706](https://wa.me/628129893706)**
📷 Instagram: **@kukuhtw**
🐦 X / Twitter: **@kukuhtw**
👍 Facebook: **[https://www.facebook.com/kukuhtw](https://www.facebook.com/kukuhtw)**
💼 LinkedIn: **[https://id.linkedin.com/in/kukuhtw](https://id.linkedin.com/in/kukuhtw)**

---

## 📄 License

**MIT**

---

## 💡 Notes

* On **Windows**, if you place FFmpeg in `C:\ffmpeg\bin`, Vibbon will **append that path** to the process `PATH` at runtime (if the folder exists). This helps run `ffmpeg.exe` without extra configuration.
* Endpoint names & fields may evolve with the implementation in `src/`. Use the examples above as a guide, then align with your actual routes.
