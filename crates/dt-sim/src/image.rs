//! Frame buffers to and from PNG, and golden-image comparison.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use dt_ui::framebuffer::{FrameBuffer, HEIGHT, WIDTH};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

/// Environment variable that makes golden tests rewrite the stored images.
pub const UPDATE_ENV: &str = "UPDATE_GOLDEN";

/// Write a frame as a 1-bit greyscale PNG (black = set pixel).
pub fn write_png(fb: &FrameBuffer, path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut enc = png::Encoder::new(BufWriter::new(File::create(path)?), WIDTH, HEIGHT);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::One);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    // PNG 1-bit greyscale: 1 = white. The frame buffer: 1 = black.
    let data: Vec<u8> = fb.as_bytes().iter().map(|b| !b).collect();
    w.write_image_data(&data).map_err(std::io::Error::other)
}

/// Read a PNG written by [`write_png`] (any 800×480 greyscale or RGB PNG
/// works; pixels darker than mid-grey count as black).
pub fn read_png(path: &Path) -> std::io::Result<FrameBuffer> {
    let mut dec = png::Decoder::new(std::io::BufReader::new(File::open(path)?));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(std::io::Error::other)?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buf).map_err(std::io::Error::other)?;
    if (info.width, info.height) != (WIDTH, HEIGHT) {
        return Err(std::io::Error::other(format!(
            "{}x{} image, expected {WIDTH}x{HEIGHT}",
            info.width, info.height
        )));
    }
    let channels = info.color_type.samples();
    let mut fb = FrameBuffer::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let i = (y as usize * info.line_size) + x as usize * channels;
            if buf[i] < 128 {
                Pixel(Point::new(x as i32, y as i32), BinaryColor::On)
                    .draw(&mut fb)
                    .ok();
            }
        }
    }
    Ok(fb)
}

/// Number of pixels that differ.
pub fn diff_count(a: &FrameBuffer, b: &FrameBuffer) -> u32 {
    a.as_bytes()
        .iter()
        .zip(b.as_bytes())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

/// Directory of stored golden images.
pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("golden")
}

/// Where mismatching frames are written for inspection.
pub fn failure_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures")
}

/// Compare `fb` with `golden/<name>.png`. With `UPDATE_GOLDEN=1` (or when
/// the golden image does not exist yet) the image is written instead. On a
/// mismatch the actual frame is saved to `target/golden-failures/` and the
/// test panics with the number of differing pixels.
pub fn assert_golden(name: &str, fb: &FrameBuffer) {
    let path = golden_dir().join(format!("{name}.png"));
    let update = std::env::var(UPDATE_ENV).is_ok_and(|v| !v.is_empty() && v != "0");
    if update || !path.exists() {
        write_png(fb, &path).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
        if !update {
            eprintln!(
                "golden image {} created; review and commit it",
                path.display()
            );
        }
        return;
    }
    let expected = read_png(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let diff = diff_count(&expected, fb);
    if diff != 0 {
        let actual = failure_dir().join(format!("{name}.actual.png"));
        write_png(fb, &actual).ok();
        panic!(
            "{name}: {diff} pixels differ from {}.\nActual frame: {}\nIf the change is intended, rerun with {UPDATE_ENV}=1.",
            path.display(),
            actual.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::primitives::{Circle, PrimitiveStyle};

    fn sample() -> FrameBuffer {
        let mut fb = FrameBuffer::new();
        Circle::new(Point::new(100, 100), 200)
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
            .draw(&mut fb)
            .unwrap();
        Pixel(Point::new(799, 479), BinaryColor::On)
            .draw(&mut fb)
            .unwrap();
        fb
    }

    #[test]
    fn png_round_trip() {
        let fb = sample();
        let path =
            std::env::temp_dir().join(format!("dt-sim-roundtrip-{}.png", std::process::id()));
        write_png(&fb, &path).unwrap();
        let back = read_png(&path).unwrap();
        assert_eq!(diff_count(&fb, &back), 0);
    }

    #[test]
    fn one_pixel_change_is_detected() {
        let a = sample();
        let mut b = sample();
        Pixel(Point::new(5, 5), BinaryColor::On)
            .draw(&mut b)
            .unwrap();
        assert_eq!(diff_count(&a, &b), 1);
    }

    #[test]
    fn golden_comparison() {
        assert_golden("harness_sample", &sample());
        // In update mode a mismatch would overwrite the golden image.
        if std::env::var(UPDATE_ENV).is_ok() {
            return;
        }
        let mut changed = sample();
        Pixel(Point::new(5, 5), BinaryColor::On)
            .draw(&mut changed)
            .unwrap();
        let r = std::panic::catch_unwind(|| assert_golden("harness_sample", &changed));
        let msg = *r.unwrap_err().downcast::<String>().unwrap();
        assert!(msg.contains("1 pixels differ"), "{msg}");
    }
}
