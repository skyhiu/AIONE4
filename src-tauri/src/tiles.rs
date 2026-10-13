//! Tile-sized copies of the game pictures (D-007) and small icons for the title (B-8).
//!
//! **Why:** Home showed the full-size `Thumbnail` files (hundreds of kB PNGs, often thousands of
//! pixels across) in 384–512 px tiles. Decoding and scaling such pictures while the row scrolls is
//! the hypothesis for D-007's freezes. The core now makes, once, a square **640 × 640 JPEG** of
//! each tile picture (centre crop, like the tile's `object-fit: cover`), and a **128 × 128 PNG**
//! of each icon (whole picture fitted, transparency kept). Home decodes these small files ahead,
//! so scrolling never decodes.
//!
//! **Where:** `<data>\Cache\Tiles\<game id>-<tile|icon>-<key>.<jpg|png>`. The key comes from the
//! source file's size and modification time, so a changed picture is made again and old copies
//! of that game are removed. The cache can be deleted at any time; it is rebuilt at the next start.
//! The game's own files are never changed (A9).

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const TILE_PX: u32 = 640;
pub const ICON_PX: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tile,
    Icon,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Tile => "tile",
            Kind::Icon => "icon",
        }
    }
    fn ext(self) -> &'static str {
        match self {
            Kind::Tile => "jpg",
            Kind::Icon => "png",
        }
    }
}

/// The result for one picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    pub path: PathBuf,
    /// The cache key (also used in the media URL, so the web view never shows a stale copy).
    pub key: String,
    /// True when it was made now; false when an existing copy was reused.
    pub made: bool,
    /// The source picture's size in pixels (only known when it was made now).
    pub source_px: Option<(u32, u32)>,
}

pub fn cache_dir(data: &Path) -> PathBuf {
    data.join("Cache").join("Tiles")
}

/// File extensions the picture slots may hold. Videos (a `portrait.mp4`, a `background.mp4`)
/// are not pictures and never become tiles or icons.
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

/// The game id as a safe file-name part (letters, digits, `-`, `_`; others become `_`).
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

/// Size and modification time of the source, as a short hex key.
pub fn key_of(src: &Path) -> Option<String> {
    let m = fs::metadata(src).ok()?;
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // FNV-1a over size and time: stable, no extra dependency.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in m.len().to_le_bytes().iter().chain(t.to_le_bytes().iter()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(format!("{h:016x}"))
}

pub fn path_for(data: &Path, id: &str, kind: Kind, key: &str) -> PathBuf {
    cache_dir(data).join(format!("{}-{}-{key}.{}", safe(id), kind.name(), kind.ext()))
}

fn decode(src: &Path) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::open(src).map_err(|e| e.to_string())?;
    if src
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ico"))
    {
        reader.set_format(ImageFormat::Ico);
    } else {
        reader = reader.with_guessed_format().map_err(|e| e.to_string())?;
    }
    reader.decode().map_err(|e| e.to_string())
}

/// Centre-crops to a square and scales to `px` (the tile's `object-fit: cover`).
pub fn square_cover(img: &DynamicImage, px: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let side = w.min(h).max(1);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop_imm(x, y, side, side)
        .resize_exact(px, px, FilterType::Lanczos3)
}

/// Fits the whole picture inside `px` × `px` (icons keep their shape and transparency).
pub fn fit(img: &DynamicImage, px: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w <= px && h <= px {
        return img.clone();
    }
    img.resize(px, px, FilterType::Lanczos3)
}

/// Makes (or reuses) the cached copy of `src` for game `id`.
pub fn ensure(data: &Path, id: &str, kind: Kind, src: &Path) -> Result<Made, String> {
    let key = key_of(src).ok_or_else(|| format!("cannot read {}", src.display()))?;
    let out = path_for(data, id, kind, &key);
    if out.is_file() {
        return Ok(Made {
            path: out,
            key,
            made: false,
            source_px: None,
        });
    }
    let img = decode(src)?;
    let source_px = Some(img.dimensions());
    fs::create_dir_all(cache_dir(data)).map_err(|e| e.to_string())?;
    let tmp = out.with_extension("tmp");
    match kind {
        Kind::Tile => {
            let rgb = square_cover(&img, TILE_PX).to_rgb8();
            let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 88)
                .encode_image(&rgb)
                .map_err(|e| e.to_string())?;
        }
        Kind::Icon => {
            fit(&img, ICON_PX)
                .to_rgba8()
                .save_with_format(&tmp, ImageFormat::Png)
                .map_err(|e| e.to_string())?;
        }
    }
    fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
    // Remove older copies of the same game and kind.
    let prefix = format!("{}-{}-", safe(id), kind.name());
    if let Ok(read) = fs::read_dir(cache_dir(data)) {
        for e in read.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(&prefix) && e.path() != out {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    Ok(Made {
        path: out,
        key,
        made: true,
        source_px,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn big_png(path: &Path, w: u32, h: u32) {
        let img = RgbImage::from_fn(w, h, |x, y| Rgb([(x % 251) as u8, (y % 241) as u8, 90]));
        img.save(path).unwrap();
    }

    #[test]
    fn makes_a_640_square_once_and_reuses_it() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("thumbnail.png");
        big_png(&src, 2400, 1600);
        let a = ensure(data, "My Game", Kind::Tile, &src).unwrap();
        assert!(a.made);
        assert_eq!(a.source_px, Some((2400, 1600)));
        let out = image::open(&a.path).unwrap();
        assert_eq!(out.dimensions(), (TILE_PX, TILE_PX));
        assert!(a.path.to_string_lossy().ends_with(".jpg"));
        let b = ensure(data, "My Game", Kind::Tile, &src).unwrap();
        assert!(!b.made, "second start reuses the copy");
        assert_eq!(a.path, b.path);
    }

    #[test]
    fn a_changed_picture_is_made_again_and_the_old_copy_removed() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("thumbnail.png");
        big_png(&src, 800, 800);
        let a = ensure(data, "G", Kind::Tile, &src).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        big_png(&src, 1000, 700);
        let b = ensure(data, "G", Kind::Tile, &src).unwrap();
        assert!(b.made);
        assert_ne!(a.path, b.path);
        assert!(!a.path.exists());
    }

    #[test]
    fn icons_keep_transparency_and_fit_128() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("icon.png");
        RgbaImage::from_fn(512, 256, |x, _| {
            Rgba([200, 0, 100, if x < 10 { 0 } else { 255 }])
        })
        .save(&src)
        .unwrap();
        let m = ensure(data, "G", Kind::Icon, &src).unwrap();
        let out = image::open(&m.path).unwrap();
        assert_eq!(out.dimensions(), (128, 64));
        // Transparency kept (scaling blends the edge a little).
        assert!(out.to_rgba8().get_pixel(0, 0)[3] < 32);
    }

    #[test]
    fn ico_files_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let src = data.join("icon.ico");
        RgbaImage::from_fn(256, 256, |_, _| Rgba([255, 70, 162, 255]))
            .save_with_format(&src, ImageFormat::Ico)
            .unwrap();
        let m = ensure(data, "G", Kind::Icon, &src).unwrap();
        assert_eq!(image::open(&m.path).unwrap().dimensions(), (128, 128));
    }

    #[test]
    fn videos_and_odd_names_are_not_pictures() {
        assert!(is_picture("thumbnail.PNG"));
        assert!(is_picture("icon.ico"));
        assert!(!is_picture("portrait.mp4"));
        assert!(!is_picture("background.webm"));
        assert_eq!(safe("Zero: Escape/1"), "Zero__Escape_1");
    }

    #[test]
    fn unreadable_pictures_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("thumbnail.png");
        fs::write(&src, b"not a picture").unwrap();
        assert!(ensure(dir.path(), "G", Kind::Tile, &src).is_err());
    }
}
