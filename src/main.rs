
/*=============================================================================
  Vibbon – Solusi Watermark Video 
  untuk Kampanye Brand & Event
  
  Programmer Profile
  Name      : Kukuh Tripamungkas Wicaksono (Kukuh TW)
  📧 Email  : kukuhtw@gmail.com
  📱 WA     : https://wa.me/628129893706
  📷 IG     : @kukuhtw
  🐦 X/Twt  : @kukuhtw
  👍 FB     : https://www.facebook.com/kukuhtw
  💼 LinkedIn: https://id.linkedin.com/in/kukuhtw
=============================================================================*/


// Menyajikan file statis (mis. folder public/assets)
use actix_files::Files;
// Menangani upload form bertipe multipart/form-data secara streaming
use actix_multipart::Multipart;
// Komponen utama Actix Web:
// - get, post: attribute macro untuk mendaftarkan handler GET/POST
// - App: builder konfigurasi aplikasi
// - Error as ActixError: alias tipe error Actix
// - HttpResponse: membentuk respons HTTP
// - HttpServer: menjalankan server HTTP async
// - Responder: trait untuk tipe yang bisa dikonversi jadi respons
use actix_web::{get, post, App, Error as ActixError, HttpResponse, HttpServer, Responder};
// Ekstensi untuk Stream (mis. .next().await) saat membaca bagian-bagian Multipart
use futures_util::StreamExt;
// Variabel statik yang diinisialisasi saat pertama kali dipakai (lazy)
// berguna untuk config global (path upload, dsb.)
use once_cell::sync::Lazy;
// Membersihkan nama file dari karakter berbahaya/ilegal sebelum disimpan
use sanitize_filename::sanitize;
// Derive trait Deserialize (serde) untuk parsing body/query ke struct
use serde::Deserialize;
use std::{
    // Cow (Clone-On-Write): menampung borrow/owned string secara efisien
    borrow::Cow,
    // Akses variabel environment (PORT, UPLOAD_DIR, dsb.)
    env,
    // Representasi string OS (nama file) yang tidak selalu UTF-8
    ffi::OsStr,
    // Tipe path untuk operasi filesystem (Path = borrowed, PathBuf = owned)
    path::{Path, PathBuf},
    // Konfigurasi stdio saat menjalankan proses anak (redirect/pipe)
    process::Stdio,
    // Mendapatkan timestamp (mis. menamai file dengan waktu sekarang)
    time::SystemTime,
};
// Tokio async I/O dan proses:
// - fs, fs::File: operasi file/direktori non-blocking
// - AsyncWriteExt: menulis buffer async (.write_all().await)
// - process::Command: menjalankan proses eksternal secara async
use tokio::{fs, fs::File, io::AsyncWriteExt, process::Command};
// Membuat identifier unik (mis. nama file sementara: <uuid>.tmp)
use uuid::Uuid;
// Mencari path executable di PATH untuk memastikan tool eksternal tersedia
use which::which;


// ================== CONFIG ==================
const MAX_DURATION_SEC: f64 = 30.0;
const OUT_WIDTH: i32 = 720;
const OUT_HEIGHT: i32 = 1280;
const CRF: i32 = 23;
const PRESET: &str = "veryfast";
const ALLOW_TRIM: bool = true;
const FILL_MODE: &str = "crop"; // "crop" | "pad"

// ================== BINARY RESOLUTION ==================
#[derive(Clone)]
struct Bins {
    ffmpeg: String,
    ffprobe: String,
}

// BINS adalah variabel global yang di-inisialisasi malas (lazy):
// closure di dalam Lazy::new hanya dieksekusi sekali saat BINS pertama kali diakses.

static BINS: Lazy<Bins> = Lazy::new(|| {
    #[cfg(target_os = "windows")]
    {
        // Sesuaikan dengan lokasi file path ffmpeg anda 
        let default_bin = Path::new("C:\\ffmpeg\\bin");
        if default_bin.exists() {
            if let Ok(old) = env::var("PATH") {
                let newp = format!("{};{}", default_bin.display(), old);
                env::set_var("PATH", newp);
            }
        }
    }

    let ffmpeg = which("ffmpeg")
        .or_else(|_| which("ffmpeg.exe"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/usr/bin/ffmpeg".to_string());

    let ffprobe = which("ffprobe")
        .or_else(|_| which("ffprobe.exe"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/usr/bin/ffprobe".to_string());

    Bins { ffmpeg, ffprobe }
});

// ================== TEMPLATE MODEL ==================
// Replace the OverlayType enum definition with this:
#[derive(Clone)]
#[allow(dead_code)]
enum OverlayType {
    Full,
    Band { h: Option<i32>, x: Option<String>, y: Option<String> },
    Logo { w: Option<i32>, h: Option<i32>, x: Option<String>, y: Option<String> },
}

#[derive(Clone)]
struct Overlay {
    path: &'static str,
    typ: OverlayType,
    start: f64,
    end: f64,
}

#[derive(Clone)]
struct Template {
    key: &'static str,
    title: &'static str,
    overlays: Vec<Overlay>,
}

static TEMPLATES: Lazy<Vec<Template>> = Lazy::new(|| {
    vec![Template {
        key: "reuni_391",
        title: "Reuni SMA 3 Jakarta • 24 Agustus 2025 (3-91)",
        overlays: vec![Overlay {
            path: "templates/2d.png",
            typ: OverlayType::Full,
            start: 0.0,
            end: 30.0,
        }],
    }]
});

// ================== HELPERS ==================
fn ensure_dirs() -> std::io::Result<()> {
    // Pakai std::fs agar tak perlu await
    for d in ["uploads", "outputs", "templates"] {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}

fn random_name(prefix: &str) -> String {
    format!("{}{}", prefix, uuid::Uuid::new_v4().simple())
}

async fn ffprobe_duration(ffprobe: &str, path: &Path) -> anyhow::Result<f64> {
    let out = Command::new(ffprobe)
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path.as_os_str())
        .stdout(Stdio::piped())
        .output()
        .await?;

    if !out.status.success() {
        anyhow::bail!("ffprobe failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let dur = s.parse::<f64>().unwrap_or(0.0);
    Ok(dur.max(0.0))
}

fn html_escape(s: &str) -> String {
    htmlescape::encode_minimal(s)
}

#[derive(Default)]
struct PostFields {
    source: Option<String>,
    template: Option<String>,
    title: Option<String>,
    upload_path: Option<PathBuf>,
    upload_ext: Option<String>,
    upload_mime: Option<String>,
}

async fn save_multipart(mut payload: Multipart) -> Result<PostFields, ActixError> {
    let mut fields = PostFields::default();

    while let Some(item) = payload.next().await {
        let mut field = item?;
        let cd = field.content_disposition().clone();
        let name = cd.get_name().unwrap_or("").to_string();

        if name == "video" {
            let filename = cd
                .get_filename()
                .map(|s| sanitize(s))
                .unwrap_or_else(|| format!("upload-{}.bin", Uuid::new_v4()));

            let ext = Path::new(&filename)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_ascii_lowercase();

            let tmp_path = PathBuf::from(format!("uploads/{}", random_name("raw_")));
            let mut f = File::create(&tmp_path).await?;
            let mut size: u64 = 0;

            while let Some(chunk) = field.next().await {
                let bytes = chunk?;
                size += bytes.len() as u64;
                f.write_all(&bytes).await?;
            }
            f.flush().await?;

            if size < 1_000 {
                return Err(actix_web::error::ErrorBadRequest(
                    "Upload video kosong/tidak valid",
                ));
            }

            fields.upload_path = Some(tmp_path);
            fields.upload_ext = Some(ext);
            // v0.6: content_type() langsung &Mime
            

fields.upload_mime = Some(
    field.content_type()
        .map(|mime| mime.essence_str().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
);



        } else {
            // field teks
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk?;
                bytes.extend_from_slice(&data);
            }
            let text = String::from_utf8(bytes).unwrap_or_default();
            match name.as_str() {
                "source" => fields.source = Some(text),
                "template" => fields.template = Some(text),
                "title" => fields.title = Some(text),
                _ => {}
            }
        }
    }

    Ok(fields)
}

// Build filter_complex graph
fn build_filter_graph(tpl: &Template) -> (String, usize) {
    let mut chains: Vec<String> = Vec::new();

    if FILL_MODE == "crop" {
        let ratio = OUT_WIDTH as f64 / OUT_HEIGHT as f64;
        chains.push(format!(
            "[0:v]scale=if(gte(a\\,{ratio})\\,-2\\,{OUT_WIDTH}):if(gte(a\\,{ratio})\\,{OUT_HEIGHT}\\,-2):flags=fast_bilinear,\
             crop={OUT_WIDTH}:{OUT_HEIGHT}:(iw-{OUT_WIDTH})/2:(ih-{OUT_HEIGHT})/2,setsar=1[base]"
        ));
    } else {
        chains.push(format!(
            "[0:v]scale={OUT_WIDTH}:{OUT_HEIGHT}:force_original_aspect_ratio=decrease:flags=fast_bilinear,\
             pad={OUT_WIDTH}:{OUT_HEIGHT}:(ow-iw)/2:(oh-ih)/2,setsar=1[base]"
        ));
    }

    let mut prev = Cow::Borrowed("base");
    let mut ov_index = 0usize;
    let mut input_count = 1usize; // 0 adalah video utama

    for ol in &tpl.overlays {
        input_count += 1;
        let in_tag = format!("{}", input_count - 1);
        ov_index += 1;
        let tag_ov = format!("ov{}", ov_index);

        match &ol.typ {
            OverlayType::Full => {
                chains.push(format!(
                    "[{in_tag}:v]scale={OUT_WIDTH}:{OUT_HEIGHT}:flags=fast_bilinear[{tag_ov}]"
                ));
                let (start, end) = (format!("{:.3}", ol.start), format!("{:.3}", ol.end));
                chains.push(format!(
                    "[{prev}][{tag_ov}]overlay=shortest=1:x=0:y=0:enable=between(t\\,{start}\\,{end})[v{ov_index}]"
                ));
            }
            OverlayType::Band { h, x, y } => {
                let hh = h.unwrap_or(160);
                chains.push(format!(
                    "[{in_tag}:v]scale={OUT_WIDTH}:{hh}:flags=fast_bilinear[{tag_ov}]"
                ));
                let x = x.clone().unwrap_or_else(|| "(main_w-w)/2".into());
                let y = y.clone().unwrap_or_else(|| "main_h-h".into());
                let (start, end) = (format!("{:.3}", ol.start), format!("{:.3}", ol.end));
                chains.push(format!(
                    "[{prev}][{tag_ov}]overlay=shortest=1:x={x}:y={y}:enable=between(t\\,{start}\\,{end})[v{ov_index}]"
                ));
            }
            OverlayType::Logo { w, h, x, y } => {
                let ww = w.unwrap_or(220);
                let hh = h.unwrap_or(-1);
                chains.push(format!(
                    "[{in_tag}:v]scale={ww}:{hh}:flags=fast_bilinear[{tag_ov}]"
                ));
                let x = x.clone().unwrap_or_else(|| "main_w-w-24".into());
                let y = y.clone().unwrap_or_else(|| "24".into());
                let (start, end) = (format!("{:.3}", ol.start), format!("{:.3}", ol.end));
                chains.push(format!(
                    "[{prev}][{tag_ov}]overlay=shortest=1:x={x}:y={y}:enable=between(t\\,{start}\\,{end})[v{ov_index}]"
                ));
            }
        }

        prev = Cow::Owned(format!("v{}", ov_index));
    }

    (chains.join(";"), ov_index)
}

// ================== HTML ==================
fn render_home(warn: Option<&str>) -> String {
    let warn_html = warn
        .map(|w| format!("<p style='color:#b00'>{}</p>", html_escape(w)))
        .unwrap_or_default();

    let mut opts = String::new();
    for t in TEMPLATES.iter() {
        opts.push_str(&format!(
            "<option value=\"{}\">{} ({})</option>",
            html_escape(t.key),
            html_escape(t.title),
            html_escape(t.key)
        ));
    }

    format!(
        r#"<!doctype html>
<html lang="id"><head>
  <meta charset="utf-8"><title>Video Twibbon</title>
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <style>
    body{{font-family:system-ui,-apple-system,Segoe UI,Roboto,Arial;color:#222;padding:24px;max-width:900px;margin:auto}}
    .card{{border:1px solid #ddd;border-radius:12px;padding:18px;margin:12px 0;background:#fff}}
    label{{display:block;margin:10px 0 6px;font-weight:600}}
    input[type=file],select,input[type=text]{{padding:10px;border:1px solid #ccc;border-radius:8px;width:100%}}
    button{{padding:12px 18px;border:0;border-radius:10px;background:#111;color:#fff;font-weight:700;cursor:pointer}}
    button:hover{{opacity:.9}}.hint{{color:#666;font-size:.9em}}code{{background:#f6f6f6;padding:2px 6px;border-radius:6px}}
  </style>
</head><body>
  <h1>Video Twibbon Generator (Rust)</h1>
  {warn_html}
  <p class="hint">Pilih sumber video: upload berkas atau rekam dari kamera. Maks 30 detik.</p>

  <form id="twb-form" class="card" method="post" enctype="multipart/form-data" action="/">
    <fieldset style="border:0;padding:0;margin:0 0 12px">
      <legend style="font-weight:700;margin-bottom:6px">Sumber Video</legend>
      <label style="margin-right:12px"><input type="radio" name="source" value="upload" checked> Upload</label>
      <label><input type="radio" name="source" value="record"> Rekam kamera</label>
    </fieldset>

    <div id="upload-pane">
      <label>Video (MP4/WEBM)</label>
      <input type="file" name="video" accept="video/mp4,video/webm,video/*">
      <p class="hint">Format disarankan: MP4 (h.264+aac) atau WEBM (vp8/9+opus).</p>
    </div>

    <div id="record-pane" hidden>
      <div style="display:grid;gap:8px">
        <video id="cam" autoplay muted playsinline style="width:360px;max-height:640px;border-radius:12px;border:1px solid #ddd;background:#000"></video>
        <div style="display:flex;gap:8px;flex-wrap:wrap">
          <button type="button" id="btnOpen" class="btn" style="background:#0a7">Nyalakan Kamera</button>
          <button type="button" id="btnRec"  class="btn" style="background:#0a7" disabled>Rekam</button>
          <button type="button" id="btnStop" class="btn" style="background:#a70" disabled>Stop</button>
          <span id="timer" class="hint" style="align-self:center">00:00</span>
        </div>
        <video id="playback" controls hidden style="width:360px;max-height:640px;border-radius:12px;border:1px solid #ddd"></video>
        <p class="hint">Rekaman otomatis berhenti di 30 detik.</p>
      </div>
    </div>

    <label style="margin-top:12px">Pilih Template</label>
    <select name="template" required>
      {opts}
    </select>

    <label>Judul Output (opsional)</label>
    <input type="text" name="title" placeholder="mis. video-twibbon">

    <button type="submit" id="btnSubmit" class="btn" style="margin-top:12px">Generate</button>
    <div id="waitNote" class="hint" style="display:none;margin-top:8px">⏳ Memproses… mohon tunggu sebentar.</div>
  </form>

<script>
/* ... JS sama persis seperti versi Anda ... (dipersingkat di sini) */
</script>
</body></html>"#
    )
}

fn render_result_page(title: &str, out_path: &str, full_cmd: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="id"><head>
  <meta charset="utf-8"><title>Hasil: {}</title>
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <style>
    body{{font-family:system-ui,-apple-system,Segoe UI,Roboto,Arial;color:#222;padding:24px;max-width:900px;margin:auto}}
    .card{{border:1px solid #ddd;border-radius:12px;padding:18px;margin:12px 0;background:#fff}}
    video{{width:360px;max-height:640px;border-radius:12px;border:1px solid #ddd}}
    a.btn{{display:inline-block;margin-top:12px;padding:12px 18px;background:#111;color:#fff;text-decoration:none;border-radius:10px;font-weight:700}}
    .mono{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:.9em;color:#333;background:#f8f8f8;border-radius:8px;padding:10px;white-space:pre-wrap}}
  </style>
</head><body>
  <h1>✅ Video berhasil dibuat</h1>
  <div class="card">
    <video controls src="{}"></video>
    <div>
      <a class="btn" href="{}" download>Download Video</a>
      <a class="btn" href="/">Buat Lagi</a>
    </div>
    <p class="mono">{}</p>
  </div>
</body></html>"#,
        html_escape(title),
        html_escape(out_path),
        html_escape(out_path),
        html_escape(full_cmd)
    )
}

// ================== ROUTES ==================
#[get("/")]
async fn home() -> impl Responder {
    let warn = {
        let mut msgs = Vec::new();
        if !Path::new(&BINS.ffmpeg).exists() && which("ffmpeg").is_err() {
            msgs.push("FFmpeg tidak ditemukan. Ubuntu: `sudo apt install ffmpeg`.");
        }
        if !Path::new(&BINS.ffprobe).exists() && which("ffprobe").is_err() {
            msgs.push("FFprobe tidak ditemukan. Ubuntu: `sudo apt install ffmpeg`.");
        }
        if msgs.is_empty() { None } else { Some(msgs.join(" ")) }
    };
    let html = render_home(warn.as_deref());
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

#[derive(Deserialize)]
struct Q {}

#[post("/")]
async fn process_upload(payload: Multipart) -> Result<impl Responder, ActixError> {
    ensure_dirs().map_err(actix_web::error::ErrorInternalServerError)?;
    let fields = save_multipart(payload).await?;

    let source = fields.source.unwrap_or_else(|| "upload".into());
    let template_key = fields
        .template
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Template wajib diisi"))?;
    let title_in = fields.title.unwrap_or_default();
    let title = if title_in.trim().is_empty() {
        let ts = humantime::format_rfc3339(SystemTime::now()).to_string();
        format!("twibbon-{}-{}", ts.replace([':', 'T', 'Z', '.'], ""), &random_name(""))
    } else {
        sanitize(&title_in)
    };

    let tpl = TEMPLATES
        .iter()
        .find(|t| t.key == template_key)
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Template tidak dikenali"))?
        .clone();

    let upload_path = fields
        .upload_path
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Upload video gagal"))?;
    let ext = fields.upload_ext.unwrap_or_default();
    let mime = fields.upload_mime.unwrap_or_default();

    // --- Normalisasi/Validasi ke MP4 (hasilkan input_path) ---
    let input_path: PathBuf = if source == "record" {
        if !["webm", "mp4"].contains(&ext.as_str())
            || !(mime.contains("video/webm")
                || mime.contains("video/mp4")
                || mime == "application/octet-stream")
        {
            return Err(actix_web::error::ErrorBadRequest("Rekaman harus WEBM/MP4."));
        }
        let input_path = PathBuf::from(format!("uploads/{}.mp4", random_name("vid_")));
        let mut cmd = Command::new(&BINS.ffmpeg);
        cmd.arg("-y")
            .arg("-i")
            .arg(&upload_path)
            .args(["-c:v", "libx264", "-preset", PRESET, "-crf", &CRF.to_string()])
            .args(["-c:a", "aac"])
            .args(["-movflags", "+faststart"])
            .arg(&input_path)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let out = cmd
            .output()
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        // bersihkan raw
        let _ = fs::remove_file(&upload_path).await;
        if !out.status.success()
            || !input_path.exists()
            || input_path.metadata().map(|m| m.len()).unwrap_or(0) < 1000
        {
            let errlog = String::from_utf8_lossy(&out.stderr);
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Gagal konversi rekaman ke MP4.\n{}",
                errlog
            )));
        }
        input_path
    } else {
        // Upload biasa: wajib MP4
        if ext.as_str() != "mp4" || !(mime.contains("video/mp4") || mime == "application/octet-stream") {
            let _ = fs::remove_file(&upload_path).await;
            return Err(actix_web::error::ErrorBadRequest("File harus MP4."));
        }
        let input_path = PathBuf::from(format!("uploads/{}.mp4", random_name("vid_")));
        fs::rename(&upload_path, &input_path)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        input_path
    };

    // ===== Proses FFmpeg utama =====
    let dur = ffprobe_duration(&BINS.ffprobe, &input_path)
        .await
        .map_err(actix_web::error::ErrorBadRequest)?;
    if dur <= 0.0 {
        let _ = fs::remove_file(&input_path).await;
        return Err(actix_web::error::ErrorBadRequest(
            "Tidak bisa membaca durasi video (ffprobe).",
        ));
    }
    let need_trim = ALLOW_TRIM && dur > (MAX_DURATION_SEC + 0.3);

    // siapkan filter graph
    let (graph, last_index) = build_filter_graph(&tpl);
    let fc_file = PathBuf::from(format!("uploads/fc_{}.txt", random_name("")));
    fs::write(&fc_file, &graph)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // build command (sekali saja)
    let out_file_name = format!(
        "{}.mp4",
        sanitize(&title)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>()
    );
    let out_file_rel = format!("outputs/{}", out_file_name);
    let out_file = PathBuf::from(&out_file_rel);

    let mut cmd = Command::new(&BINS.ffmpeg);
    cmd.arg("-y").arg("-i").arg(&input_path);
    for ol in &tpl.overlays {
        cmd.args(["-loop", "1", "-framerate", "30"]).arg("-i").arg(ol.path);
    }
    if need_trim {
        cmd.arg("-t").arg(format!("{}", MAX_DURATION_SEC));
    }
    cmd.arg("-filter_complex_script")
        .arg(&fc_file)
        .args(["-c:v", "libx264"])
        .args(["-crf", &CRF.to_string()])
        .args(["-preset", PRESET])
        .args(["-pix_fmt", "yuv420p"])
        .args(["-c:a", "aac"])
        .args(["-movflags", "+faststart"])
        .arg("-shortest")
        .args(["-map", &format!("[v{}]", last_index)])
        .args(["-map", "0:a?"])
        .arg(&out_file)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());

    let full_cmd_for_view = {
        let mut s = format!("{} -y -i \"{}\" ", &BINS.ffmpeg, input_path.display());
        for ol in &tpl.overlays {
            s.push_str("-loop 1 -framerate 30 -i ");
            s.push('"');
            s.push_str(ol.path);
            s.push('"');
            s.push(' ');
        }
        if need_trim {
            s.push_str(&format!("-t {} ", MAX_DURATION_SEC));
        }
        s.push_str(&format!(
            "-filter_complex_script \"{}\" -c:v libx264 -crf {} -preset {} -pix_fmt yuv420p -c:a aac -movflags +faststart -shortest -map [v{}] -map 0:a? \"{}\"",
            fc_file.display(), CRF, PRESET, last_index, out_file.display()
        ));
        s
    };

    let out = cmd
        .output()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let _ = fs::remove_file(&fc_file).await;
    let _ = fs::remove_file(&input_path).await;

    if !out.status.success()
        || !out_file.exists()
        || out_file.metadata().map(|m| m.len()).unwrap_or(0) < 1000
    {
        let errlog = String::from_utf8_lossy(&out.stderr);
        let debug_html = format!(
            "<h3>Gagal generate video</h3><pre>{}</pre><pre>--- filter graph ---\n{}</pre><pre>{}</pre>",
            html_escape(&full_cmd_for_view),
            html_escape(&graph),
            html_escape(&errlog),
        );
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(debug_html));
    }

    let page = render_result_page(&title, &format!("/{}", out_file_rel), &full_cmd_for_view);
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page))
}

// ================== MAIN ==================
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    ensure_dirs().ok();

    println!(
        "FFmpeg: {}\nFFprobe: {}\nServing at: http://127.0.0.1:8080/",
        BINS.ffmpeg, BINS.ffprobe
    );

    HttpServer::new(|| {
        App::new()
            .service(home)
            .service(process_upload)
            .service(Files::new("/outputs", "outputs").show_files_listing())
            .service(Files::new("/templates", "templates").show_files_listing())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
