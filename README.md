# Vibbon — Solusi Watermark Video untuk Kampanye Brand & Event

**Vibbon** adalah web service kecil berbasis **Rust + Actix** yang memproses **video UGC** (user-generated content) dan menempelkan **frame PNG transparan / watermark** secara otomatis menggunakan **FFmpeg**. Cocok untuk kebutuhan **kampanye brand**, **event organizer**, maupun **UGC activation** di TikTok/IG Reels/YouTube Shorts.

> “Twibbon versi video” — lebih engaging, durasi fleksibel, siap pakai untuk social media.

---

## ✨ Fitur Utama

* **Overlay PNG Transparan** — menempelkan frame/logo/watermark ke video pengguna.
* **Output Portrait Siap Sosmed** — default **720×1280 (9:16)**.
* **Kompresi Efisien** — kontrol **CRF** dan **preset** FFmpeg (default ramah performa).
* **Batasi Durasi** — potong otomatis ke durasi maksimum (mis. 30 detik) agar ringan.
* **UI Sederhana + API** — unggah via halaman web atau kirim lewat HTTP `multipart/form-data`.
* **Lintas OS** — Windows, Linux, macOS (butuh FFmpeg di PATH).
* **Aman Nama Berkas** — sanitasi nama file upload untuk mencegah karakter berbahaya.

---

## 🏗️ Arsitektur Singkat

* **Web server**: \[Actix Web] + \[actix-multipart] untuk upload.
* **Async runtime**: Tokio.
* **Media engine**: FFmpeg (dipanggil sebagai proses eksternal).
* **Template**: HTML di folder `templates/` (front-end sederhana).
* **Utilitas**: `once_cell`, `uuid`, `sanitize-filename`, `futures-util`, `which`.

> Di Windows, Vibbon otomatis menambahkan `C:\ffmpeg\bin` ke `PATH` proses jika folder itu ada — memudahkan eksekusi `ffmpeg.exe` tanpa set PATH manual.

---

## 🧰 Prasyarat

* **Rust** (stable)
* **FFmpeg**:

  * **Windows**: ekstrak ke `C:\ffmpeg\bin` (atau pastikan `ffmpeg.exe` ada di PATH).
  * **Ubuntu/Debian**: `sudo apt-get install ffmpeg`
  * **macOS**: `brew install ffmpeg`

---

## 🚀 Cara Menjalankan (Development)

```bash
# 1) Clone
git clone https://github.com/kukuhtw/vibbon_rust.git
cd vibbon_rust

# 2) Jalankan (dev)
cargo run

# atau build release
cargo build --release
./target/release/vibbon_rust
```

Secara default server akan berjalan di `http://localhost:8080` (ubah sesuai konfigurasi Anda).

---

## 🌐 Cara Pakai

### 1) Via Halaman Web

1. Buka `http://localhost:8080`
2. Unggah video (MP4/MOV/WEBM) — ideal < 30 detik.
3. Pilih **template frame PNG** bila tersedia (atau pakai default).
4. Klik **Render** → unduh hasilnya (MP4) dan bagikan ke sosmed.

### 2) Via API (HTTP Multipart)

Contoh `curl` (endpoint contoh; sesuaikan dengan route di kode):

```bash
curl -X POST http://localhost:8080/api/render \
  -F "video=@/path/ke/video.mp4" \
  -F "overlay=@/path/ke/frame.png" \
  -o output.mp4
```

**Field form yang umum:**

* `video` — file video input (wajib).
* `overlay` — file PNG transparan (opsional jika ada default).
* (Opsional, tergantung implementasi) `layout=crop|pad`, `max_duration=30`, `width=720`, `height=1280`, `crf=23`, `preset=veryfast`.

> Nama endpoint & parameter persisnya silakan cek di `src/` (controller/handler Actix). README ini menyediakan skema umum agar mudah diadopsi.

---

## ⚙️ Konfigurasi

Beberapa konstanta umum yang bisa Anda temukan di kode (sesuaikan nilai di sumber):

* `MAX_DURATION_SEC` — batas durasi output (mis. `30.0`).
* `OUT_WIDTH`, `OUT_HEIGHT` — resolusi output (mis. `720×1280`).
* `CRF`, `PRESET` — kualitas & kecepatan encoding FFmpeg.
* `ALLOW_TRIM` — pemotongan otomatis bila input lebih panjang dari batas.

> Tips: Anda bisa mengekspose konstanta ini sebagai **env var** di masa depan, atau menyediakan **query/form params** agar bisa dikontrol per request.

---

## 🗂️ Struktur Direktori (ringkas)

```
vibbon_rust/
├─ src/                # kode Rust (Actix, handler, ffmpeg runner)
├─ templates/          # HTML sederhana (form upload, dsb.)
├─ Cargo.toml
└─ README.md
```

---

## 🧪 Contoh Alur FFmpeg (konseptual)

Filter dasar overlay (ilustrasi, bisa berbeda di kode):

```bash
ffmpeg -i input.mp4 -i overlay.png \
  -filter_complex "[0:v]scale=720:1280:force_original_aspect_ratio=increase,crop=720:1280[bg];[bg][1:v]overlay=(W-w)/2:(H-h)/2" \
  -c:v libx264 -crf 23 -preset veryfast -c:a copy output.mp4
```

* **Mode crop**: scale-to-cover lalu crop ke 9:16.
* **Mode pad**: scale-to-fit lalu letterbox (gunakan `pad` filter).

---

## 🔒 Keamanan & Batasan

* **Nama file** disanitasi (hindari karakter berbahaya).
* **Ukuran file**: batasi di reverse proxy/web server Anda bila perlu.
* **Durasi**: gunakan `MAX_DURATION_SEC` untuk menghindari job berat.
* **Temporary files**: pastikan folder `uploads/` (atau sejenis) memiliki izin tulis dan dibersihkan berkala.

---

## 🐳 (Opsional) Menjalankan dengan Docker

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

* [ ] Parameterisasi penuh via **ENV** (port, resolusi, crf, preset, max duration).
* [ ] **Multi-template** (pilih frame dari daftar).
* [ ] **Text overlay** dinamis (judul kampanye, nama peserta).
* [ ] **Queue/worker** untuk antrian job.
* [ ] **Docker Compose** + reverse proxy.
* [ ] **Unit test** ffmpeg command builder.

---

## 🤝 Kontribusi

Kontribusi terbuka!
Silakan buat **issue** untuk bug/feature request, atau **pull request** bila ingin langsung mengirim perubahan.

---

## 👤 Pembuat

**Kukuh TW**
📧 Email: **[kukuhtw@gmail.com](mailto:kukuhtw@gmail.com)**
📱 WhatsApp: **[https://wa.me/628129893706](https://wa.me/628129893706)**
📷 Instagram: **@kukuhtw**
🐦 X / Twitter: **@kukuhtw**
👍 Facebook: **[https://www.facebook.com/kukuhtw](https://www.facebook.com/kukuhtw)**
💼 LinkedIn: **[https://id.linkedin.com/in/kukuhtw](https://id.linkedin.com/in/kukuhtw)**

---

## 📄 Lisensi

**MIT** (atau sesuaikan bila Anda menginginkan lisensi lain).

---

## 💡 Catatan

* Di **Windows**, jika Anda menaruh FFmpeg di `C:\ffmpeg\bin`, Vibbon akan **menambahkan path tersebut** ke `PATH` proses saat runtime (bila folder ada). Ini mempermudah pemanggilan `ffmpeg.exe` tanpa konfigurasi tambahan.
* Penamaan **endpoint & field** dapat berubah mengikuti implementasi di `src/`. Gunakan contoh di atas sebagai acuan, lalu sesuaikan dengan rute aktual pada kode Anda.


