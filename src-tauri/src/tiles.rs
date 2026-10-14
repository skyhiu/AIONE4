//! Tile-sized copies of the game pictures (D-007, D-010, D-012, B-13).
//!
//! **What a tile shows (B-13, rev. k):** the game's `Icon` (the largest size inside an `.ico`),
//! **filling** the square tile. Without an icon picture: another picture (`Thumbnail`,
//! `Portrait`, `Hero`) **fitted whole** inside the tile, the empty space filled by a blurred,
//! darkened copy of the same picture (never cropped). Without any picture: a plain tile with the
//! name (drawn by the UI).
//!
//! **Copies (D-007):** the core makes one **640 × 640 JPEG** per game in
//! `<data>\Cache\Tiles\<game id>-tile-<key>.jpg`; the key comes from the source's size, time and
//! the mode, so a changed picture is made again and old copies are removed. **D-010:** copies are
//! made in the background after Home is shown (Home shows the original picture until then), and
//! each copy logs its decode / scale / encode / write times.
//!
//! **ICO files (D-012):** AIONE reads the ICO directory itself and decodes the largest entry first:
//! an embedded PNG in **any** colour type (RGB, palette, grey; the `image` library's own ICO reader
//! accepts only RGBA PNGs), or a BMP entry through the library; if an entry fails, the next size.
//!
//! **Failures:** recorded in `Cache\Tiles\failed.json` with the source's key; an unchanged source
//! that failed before is not tried again at every start (it is reported as skipped). The game's
//! own files are never changed (A9).

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, RgbImage};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, UNIX_EPOCH};

pub const TILE_PX: u32 = 640;
const FAILED_FILE: &str = "failed.json";
/// The scaling filter: CatmullRom (sharp, about 25 % cheaper than Lanczos3; measured).
const FILTER: FilterType = FilterType::CatmullRom;

/// How a source picture becomes a tile (B-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The icon fills the square (centre crop if it isn't square).
    Fill,
    /// Fitted whole over a blurred, darkened copy of itself (never cropped).
    Fit,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Fill => "fill",
            Mode::Fit => "fit",
        }
    }
}

/// Time spent per step of one copy (D-010).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Timing {
    pub decode_ms: f64,
    pub scale_ms: f64,
    pub encode_ms: f64,
    pub write_ms: f64,
}

impl Timing {
    pub fn total(&self) -> f64 {
        self.decode_ms + self.scale_ms + self.encode_ms + self.write_ms
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub path: PathBuf,
    pub key: String,
    /// True when made now; false when an existing copy was reused.
    pub made: bool,
    /// The decoded picture's size, and what was read (e.g. "ICO entry 256×256 PNG").
    pub source_px: Option<(u32, u32)>,
    pub source_note: String,
    pub timing: Option<Timing>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileError {
    /// Tried now and failed (recorded).
    Failed(String),
    /// Failed before and unchanged since: not tried again.
    Skipped(String),
}

pub fn cache_dir(data: &Path) -> PathBuf {
    data.join("Cache").join("Tiles")
}

/// File extensions the picture slots may hold (videos never become tiles).
pub fn is_picture(file: &str) -> bool {
    matches!(
        Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "ico")
    )
}

fn safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Size, modification time and mode of the source, as a short hex key.
pub fn key_of(src: &Path, mode: Mode) -> Option<String> {
    let m = fs::metadata(src).ok()?;
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let tag = [if mode == Mode::Fill { 1u8 } else { 2u8 }];
    for b in m
        .len()
        .to_le_bytes()
        .iter()
        .chain(t.to_le_bytes().iter())
        .chain(tag.iter())
    {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(format!("{h:016x}"))
}

pub fn path_for(data: &Path, id: &str, key: &str) -> PathBuf {
    cache_dir(data).join(format!("{}-tile-{key}.jpg", safe(id)))
}

/// The cached copy for this source, if it already exists (no work done).
pub fn existing(data: &Path, id: &str, src: &Path, mode: Mode) -> Option<String> {
    let key = key_of(src, mode)?;
    path_for(data, id, &key).is_file().then_some(key)
}

// ---------------------------------------------------------------------------------------
// ICO (D-012)

#[derive(Debug, Clone)]
struct IcoEntry {
    w: u32,
    h: u32,
    raw: [u8; 16],
    data: Vec<u8>,
}

const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn ico_entries(bytes: &[u8]) -> Result<Vec<IcoEntry>, String> {
    if bytes.len() < 6 || bytes[0..4] != [0, 0, 1, 0] {
        return Err("not an icon file".into());
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut out = Vec::new();
    for i in 0..count {
        let at = 6 + i * 16;
        let Some(e) = bytes.get(at..at + 16) else {
            break;
        };
        let size = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
        let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
        let Some(data) = bytes.get(off..off.saturating_add(size)) else {
            continue;
        };
        let mut raw = [0u8; 16];
        raw.copy_from_slice(e);
        // Width / height 0 mean 256 (or more, for an embedded PNG: read its header).
        let (mut w, mut h) = (
            if e[0] == 0 { 256 } else { e[0] as u32 },
            if e[1] == 0 { 256 } else { e[1] as u32 },
        );
        if data.len() >= 24 && data[..8] == PNG_SIG {
            w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        }
        out.push(IcoEntry {
            w,
            h,
            raw,
            data: data.to_vec(),
        });
    }
    if out.is_empty() {
        return Err("the icon file holds no readable entry".into());
    }
    // Largest first.
    out.sort_by_key(|e| std::cmp::Reverse(e.w as u64 * e.h as u64));
    Ok(out)
}

fn decode_ico_entry(e: &IcoEntry) -> Result<(DynamicImage, &'static str), String> {
    if e.data.len() >= 8 && e.data[..8] == PNG_SIG {
        // Any colour type (the image library's ICO reader refuses non-RGBA PNGs: D-012).
        return image::load_from_memory_with_format(&e.data, ImageFormat::Png)
            .map(|i| (i, "PNG"))
            .map_err(|err| err.to_string());
    }
    // A BMP entry: an ICO with only this entry, through the library.
    let mut one = vec![0u8, 0, 1, 0, 1, 0];
    let mut raw = e.raw;
    raw[12..16].copy_from_slice(&22u32.to_le_bytes());
    one.extend_from_slice(&raw);
    one.extend_from_slice(&e.data);
    image::load_from_memory_with_format(&one, ImageFormat::Ico)
        .map(|i| (i, "BMP"))
        .map_err(|err| err.to_string())
}

/// Decodes an ICO: the largest entry that can be read. Returns the picture and a note.
pub fn decode_ico(bytes: &[u8]) -> Result<(DynamicImage, String), String> {
    let entries = ico_entries(bytes)?;
    let mut errors = Vec::new();
    for e in &entries {
        match decode_ico_entry(e) {
            Ok((img, kind)) => {
                let note = format!(
                    "ICO entry {}×{} {kind} (largest readable of {})",
                    e.w,
                    e.h,
                    entries.len()
                );
                return Ok((img, note));
            }
            Err(err) => errors.push(format!("{}×{}: {err}", e.w, e.h)),
        }
    }
    Err(format!("no entry could be read ({})", errors.join("; ")))
}

fn decode(src: &Path) -> Result<(DynamicImage, String), String> {
    let is_ico = src
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ico"));
    if is_ico {
        let bytes = fs::read(src).map_err(|e| e.to_string())?;
        return decode_ico(&bytes);
    }
    let img = ImageReader::open(src)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    Ok((img, "picture".into()))
}

// ---------------------------------------------------------------------------------------
// Composition (B-13)

/// Fill: centre-crop to a square and scale to `px`.
pub fn fill(img: &DynamicImage, px: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    let side = w.min(h).max(1);
    img.crop_imm((w - side) / 2, (h - side) / 2, side, side)
        .resize_exact(px, px, FILTER)
        .to_rgb8()
}

/// Fit: the whole picture, centred, over a blurred and darkened copy of itself.
/// The blur is made cheaply: the picture shrunk to 24 px and smoothly scaled back up.
pub fn fit(img: &DynamicImage, px: u32) -> RgbImage {
    let small = img.resize_to_fill(24, 24, FilterType::Triangle);
    let mut bg = small.resize_exact(px, px, FilterType::Triangle).to_rgb8();
    for p in bg.pixels_mut() {
        for c in p.0.iter_mut() {
            *c = (*c as f32 * 0.45) as u8;
        }
    }
    let fg = img.resize(px, px, FILTER).to_rgba8();
    let x = (px - fg.width()) / 2;
    let y = (px - fg.height()) / 2;
    let mut out = DynamicImage::ImageRgb8(bg).to_rgba8();
    image::imageops::overlay(&mut out, &fg, x as i64, y as i64);
    DynamicImage::ImageRgba8(out).to_rgb8()
}

// ---------------------------------------------------------------------------------------
// Failures (not retried while unchanged)

static FAILED_LOCK: Mutex<()> = Mutex::new(());

fn read_failed(data: &Path) -> BTreeMap<String, (String, String)> {
    let p = cache_dir(data).join(FAILED_FILE);
    fs::read(&p)
        .ok()
        .and_then(|b| serde_json::from_slice::<BTreeMap<String, (String, String)>>(&b).ok())
        .unwrap_or_default()
}

fn write_failed(data: &Path, map: &BTreeMap<String, (String, String)>) {
    let _ = fs::create_dir_all(cache_dir(data));
    let _ = fs::write(
        cache_dir(data).join(FAILED_FILE),
        serde_json::to_vec_pretty(map).unwrap_or_default(),
    );
}

fn record(data: &Path, id: &str, key: &str, reason: Option<&str>) {
    let _guard = FAILED_LOCK.lock();
    let mut map = read_failed(data);
    let changed = match reason {
        Some(r) => map
            .insert(safe(id), (key.to_string(), r.to_string()))
            .is_none_or(|old| old.0 != key),
        None => map.remove(&safe(id)).is_some(),
    };
    if changed {
        write_failed(data, &map);
    }
}

/// Makes (or reuses) the tile copy of `src` for game `id`.
pub fn ensure(data: &Path, id: &str, src: &Path, mode: Mode) -> Result<Made, TileError> {
    let key = key_of(src, mode)
        .ok_or_else(|| TileError::Failed(format!("cannot read {}", src.display())))?;
    let out = path_for(data, id, &key);
    if out.is_file() {
        return Ok(Made {
            path: out,
            key,
            made: false,
            source_px: None,
            source_note: String::new(),
            timing: None,
        });
    }
    {
        let _guard = FAILED_LOCK.lock();
        if let Some((k, reason)) = read_failed(data).get(&safe(id)) {
            if *k == key {
                return Err(TileError::Skipped(reason.clone()));
            }
        }
    }
    let mut timing = Timing::default();
    let t = Instant::now();
    let (img, note) = match decode(src) {
        Ok(v) => v,
        Err(e) => {
            record(data, id, &key, Some(&e));
            return Err(TileError::Failed(e));
        }
    };
    timing.decode_ms = t.elapsed().as_secs_f64() * 1000.0;
    let source_px = Some(img.dimensions());
    let t = Instant::now();
    let rgb = match mode {
        Mode::Fill => fill(&img, TILE_PX),
        Mode::Fit => fit(&img, TILE_PX),
    };
    timing.scale_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let mut bytes = Vec::with_capacity(96 * 1024);
    if let Err(e) =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 88).encode_image(&rgb)
    {
        record(data, id, &key, Some(&e.to_string()));
        return Err(TileError::Failed(e.to_string()));
    }
    timing.encode_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(cache_dir(data))?;
        let tmp = out.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &out)
    };
    if let Err(e) = write() {
        return Err(TileError::Failed(e.to_string()));
    }
    // Remove older copies of the same game (any earlier key or alpha.5's icon copies).
    let prefix = format!("{}-", safe(id));
    if let Ok(read) = fs::read_dir(cache_dir(data)) {
        for e in read.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(&prefix) && e.path() != out {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    timing.write_ms = t.elapsed().as_secs_f64() * 1000.0;
    record(data, id, &key, None);
    Ok(Made {
        path: out,
        key,
        made: true,
        source_px,
        source_note: note,
        timing: Some(timing),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn png_bytes(img: DynamicImage) -> Vec<u8> {
        let mut v = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut v), ImageFormat::Png)
            .unwrap();
        v
    }

    /// An ICO built by hand with the given PNG entries (as real icons are made).
    fn ico(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut head = vec![0u8, 0, 1, 0];
        head.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        let mut off = 6 + 16 * entries.len();
        let mut body = Vec::new();
        for e in entries {
            head.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0]);
            head.extend_from_slice(&(e.len() as u32).to_le_bytes());
            head.extend_from_slice(&(off as u32).to_le_bytes());
            off += e.len();
            body.extend_from_slice(e);
        }
        head.extend(body);
        head
    }

    #[test]
    fn an_icon_fills_the_tile_once_and_is_reused() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("icon.png");
        RgbaImage::from_fn(512, 512, |x, y| {
            Rgba([(x % 250) as u8, (y % 250) as u8, 90, 255])
        })
        .save(&src)
        .unwrap();
        let a = ensure(data, "My Game", &src, Mode::Fill).unwrap();
        assert!(a.made);
        assert_eq!(a.source_px, Some((512, 512)));
        assert!(a.timing.unwrap().total() > 0.0);
        assert_eq!(
            image::open(&a.path).unwrap().dimensions(),
            (TILE_PX, TILE_PX)
        );
        let b = ensure(data, "My Game", &src, Mode::Fill).unwrap();
        assert!(!b.made && b.timing.is_none());
        assert_eq!(existing(data, "My Game", &src, Mode::Fill), Some(a.key));
    }

    /// B-13: a wide thumbnail is never cropped: the whole picture sits in the middle, the empty
    /// bands above and below show a blurred, darkened copy.
    #[test]
    fn a_wide_picture_is_fitted_whole_over_a_blurred_copy() {
        let src = DynamicImage::ImageRgb8(RgbImage::from_fn(920, 430, |x, _| {
            if x < 20 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 0, 255])
            }
        }));
        let out = fit(&src, TILE_PX);
        assert_eq!(out.dimensions(), (640, 640));
        // The picture's left edge (red, the first 20 px of 920) is still there: not cropped.
        let red = out.get_pixel(5, 320);
        assert!(red[0] > 200 && red[2] < 60, "{red:?}");
        // Above the picture: dark blurred blue, not black and not the bright picture.
        let band = out.get_pixel(320, 40);
        assert!(band[2] > 40 && band[2] < 140, "{band:?}");
    }

    /// D-012: an ICO whose PNG isn't RGBA. The image library's own reader refuses it (the cause
    /// QA confirmed); AIONE's reader takes it.
    #[test]
    fn ico_with_an_rgb_or_palette_png_is_read() {
        let rgb = png_bytes(DynamicImage::ImageRgb8(RgbImage::from_fn(
            300,
            300,
            |_, _| Rgb([20, 200, 90]),
        )));
        let bytes = ico(&[rgb]);
        assert!(image::load_from_memory_with_format(&bytes, ImageFormat::Ico).is_err());
        let (img, note) = decode_ico(&bytes).unwrap();
        assert_eq!(img.dimensions(), (300, 300));
        assert!(note.starts_with("ICO entry 300×300 PNG"), "{note}");
        let grey = png_bytes(DynamicImage::ImageLuma8(image::GrayImage::from_fn(
            64,
            64,
            |_, _| image::Luma([128]),
        )));
        assert_eq!(decode_ico(&ico(&[grey])).unwrap().0.dimensions(), (64, 64));
    }

    #[test]
    fn the_largest_readable_ico_entry_wins() {
        let small = png_bytes(DynamicImage::ImageRgba8(RgbaImage::from_fn(
            32,
            32,
            |_, _| Rgba([1, 2, 3, 255]),
        )));
        let big = png_bytes(DynamicImage::ImageRgba8(RgbaImage::from_fn(
            256,
            256,
            |_, _| Rgba([1, 2, 3, 255]),
        )));
        let mut broken = big.clone();
        broken.truncate(60); // a damaged large entry
        let (img, _) = decode_ico(&ico(&[small.clone(), big])).unwrap();
        assert_eq!(img.width(), 256);
        let (img, note) = decode_ico(&ico(&[broken, small])).unwrap();
        assert_eq!(img.width(), 32, "falls back to the next size: {note}");
    }

    #[test]
    fn a_failed_source_is_not_retried_until_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("icon.ico");
        fs::write(&src, b"not an icon").unwrap();
        assert!(matches!(
            ensure(data, "G", &src, Mode::Fill),
            Err(TileError::Failed(_))
        ));
        assert!(matches!(
            ensure(data, "G", &src, Mode::Fill),
            Err(TileError::Skipped(_))
        ));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let good = png_bytes(DynamicImage::ImageRgba8(RgbaImage::from_fn(
            64,
            64,
            |_, _| Rgba([9, 9, 9, 255]),
        )));
        fs::write(&src, ico(&[good])).unwrap();
        let m = ensure(data, "G", &src, Mode::Fill).unwrap();
        assert!(m.made);
        assert!(
            read_failed(data).is_empty(),
            "the record is cleared after success"
        );
    }

    #[test]
    fn a_changed_picture_is_made_again_and_old_copies_removed() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("icon.png");
        RgbaImage::new(64, 64).save(&src).unwrap();
        let a = ensure(data, "G", &src, Mode::Fill).unwrap();
        fs::write(cache_dir(data).join("G-icon-0123.png"), b"alpha.5 copy").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        RgbaImage::new(80, 80).save(&src).unwrap();
        let b = ensure(data, "G", &src, Mode::Fill).unwrap();
        assert!(b.made && a.path != b.path);
        assert!(!a.path.exists());
        assert!(!cache_dir(data).join("G-icon-0123.png").exists());
    }

    #[test]
    fn videos_and_odd_names_are_not_pictures() {
        assert!(is_picture("icon.ICO"));
        assert!(!is_picture("portrait.mp4"));
        assert_eq!(safe("Zero: Escape/1"), "Zero__Escape_1");
    }
}
