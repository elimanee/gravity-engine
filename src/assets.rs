//! Image decoding: raster formats, animated GIFs and SVGs, all funnelled into
//! a single CPU-side representation. Nothing in here touches the GPU, so it is
//! safe to call from worker threads.

use image::{imageops::FilterType, RgbaImage};

/// Decoded, resized image data ready to be uploaded as textures.
pub struct Decoded {
    pub frames: Vec<RgbaImage>,
    /// Per-frame delay in milliseconds (empty for still images).
    pub delays_ms: Vec<u16>,
    /// Normalised convex-hull sample points in [-0.5, 0.5]², or `None` when
    /// the image is fully opaque (a box collider is then used).
    pub hull: Option<Vec<[f32; 2]>>,
}

#[cfg(test)]
impl Decoded {
    pub fn width(&self) -> u32 {
        self.frames[0].width()
    }
    pub fn height(&self) -> u32 {
        self.frames[0].height()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gif,
    Svg,
    Raster,
}

/// Guess the kind of image from magic bytes, falling back to the file name.
pub fn sniff(name: &str, data: &[u8]) -> Kind {
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Kind::Gif;
    }
    let head = &data[..data.len().min(256)];
    let text = String::from_utf8_lossy(head);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".svg")
        || lower.ends_with(".svgz")
        || text.starts_with("<svg")
        || ((text.starts_with("<?xml") || text.starts_with("<!--")) && contains_svg(data))
    {
        return Kind::Svg;
    }
    Kind::Raster
}

fn contains_svg(data: &[u8]) -> bool {
    data.windows(4).take(4096).any(|w| w == b"<svg")
}

/// Decode `data` so that its longest side is at most `max_px`.
/// Vector images are scaled *up* to `max_px` too when `upscale_vector` is set.
pub fn decode(name: &str, data: &[u8], max_px: u32, upscale_vector: bool) -> Option<Decoded> {
    match sniff(name, data) {
        Kind::Gif => decode_gif(data, max_px),
        Kind::Svg => decode_svg(data, max_px, upscale_vector),
        Kind::Raster => decode_raster(data, max_px),
    }
}

/// Read a file and decode it.
pub fn decode_file(path: &str, max_px: u32, upscale_vector: bool) -> Option<Decoded> {
    let data = std::fs::read(path).ok()?;
    decode(path, &data, max_px, upscale_vector)
}

fn fit(w: u32, h: u32, max_px: u32, allow_up: bool) -> (u32, u32) {
    let mut scale = max_px as f32 / w.max(h).max(1) as f32;
    if !allow_up {
        scale = scale.min(1.0);
    }
    (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1))
}

fn resize(img: &RgbaImage, max_px: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (nw, nh) = fit(w, h, max_px, false);
    if (nw, nh) == (w, h) {
        img.clone()
    } else {
        image::imageops::resize(img, nw, nh, FilterType::Triangle)
    }
}

fn decode_raster(data: &[u8], max_px: u32) -> Option<Decoded> {
    let dynimg = image::load_from_memory(data).ok()?;
    let has_alpha = dynimg.color().has_alpha();
    let img = resize(&dynimg.into_rgba8(), max_px);
    let hull = if has_alpha { compute_hull(&[&img]) } else { None };
    Some(Decoded { frames: vec![img], delays_ms: vec![], hull })
}

fn decode_gif(data: &[u8], max_px: u32) -> Option<Decoded> {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut dec = opts.read_info(std::io::Cursor::new(data)).ok()?;
    let (w, h) = (dec.width() as usize, dec.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }

    let mut canvas = vec![0u8; w * h * 4];
    let mut frames = Vec::new();
    let mut delays = Vec::new();

    while let Ok(Some(frame)) = dec.read_next_frame() {
        let (fx, fy) = (frame.left as usize, frame.top as usize);
        let (fw, fh) = (frame.width as usize, frame.height as usize);
        let before = (frame.dispose == gif::DisposalMethod::Previous).then(|| canvas.clone());

        for row in 0..fh {
            for col in 0..fw {
                let (dx, dy) = (fx + col, fy + row);
                if dx >= w || dy >= h {
                    continue;
                }
                let s = (row * fw + col) * 4;
                if s + 3 < frame.buffer.len() && frame.buffer[s + 3] > 0 {
                    let d = (dy * w + dx) * 4;
                    canvas[d..d + 4].copy_from_slice(&frame.buffer[s..s + 4]);
                }
            }
        }

        let full = RgbaImage::from_raw(w as u32, h as u32, canvas.clone())?;
        frames.push(resize(&full, max_px));
        delays.push((frame.delay.max(2)).saturating_mul(10));

        match frame.dispose {
            gif::DisposalMethod::Background => {
                for row in fy..(fy + fh).min(h) {
                    for col in fx..(fx + fw).min(w) {
                        let d = (row * w + col) * 4;
                        canvas[d..d + 4].fill(0);
                    }
                }
            }
            gif::DisposalMethod::Previous => {
                if let Some(prev) = before {
                    canvas = prev;
                }
            }
            _ => {}
        }
    }

    if frames.is_empty() {
        return None;
    }
    let refs: Vec<&RgbaImage> = frames.iter().collect();
    let hull = compute_hull(&refs);
    if frames.len() == 1 {
        delays.clear();
    }
    Some(Decoded { frames, delays_ms: delays, hull })
}

fn decode_svg(data: &[u8], max_px: u32, upscale: bool) -> Option<Decoded> {
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(data, &opts).ok()?;
    let size = tree.size();
    let (sw, sh) = (size.width(), size.height());
    let (ow, oh) = fit(sw.ceil() as u32, sh.ceil() as u32, max_px, upscale);
    let mut pm = resvg::tiny_skia::Pixmap::new(ow, oh)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(ow as f32 / sw, oh as f32 / sh), &mut pm.as_mut());
    // tiny_skia is premultiplied; textures expect straight alpha.
    let mut bytes = pm.take();
    for px in bytes.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            for c in &mut px[..3] {
                *c = (*c as f32 / a).min(255.0) as u8;
            }
        }
    }
    let img = RgbaImage::from_raw(ow, oh, bytes)?;
    let hull = compute_hull(&[&img]);
    Some(Decoded { frames: vec![img], delays_ms: vec![], hull })
}

/// Sample opaque pixels (union over all frames) and return them normalised to
/// [-0.5, 0.5]². Returns `None` when every sample is opaque or too few are.
pub fn compute_hull(frames: &[&RgbaImage]) -> Option<Vec<[f32; 2]>> {
    let first = frames.first()?;
    let (w, h) = first.dimensions();
    if w < 2 || h < 2 {
        return None;
    }
    let step = (w.min(h) / 48).max(1) as usize;
    let mut pts = Vec::new();
    let mut total = 0usize;
    for y in (0..h as usize).step_by(step) {
        for x in (0..w as usize).step_by(step) {
            total += 1;
            let opaque = frames.iter().any(|f| f.get_pixel(x as u32, y as u32)[3] > 20);
            if opaque {
                let u = x as f32 / (w as f32 - 1.0) - 0.5;
                let v = 0.5 - y as f32 / (h as f32 - 1.0);
                pts.push([u, v]);
            }
        }
    }
    if pts.len() < 3 || pts.len() == total {
        None
    } else {
        Some(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(img: &RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn sniff_kinds() {
        assert_eq!(sniff("x.bin", b"GIF89a...."), Kind::Gif);
        assert_eq!(sniff("x.bin", b"<svg xmlns=''></svg>"), Kind::Svg);
        assert_eq!(sniff("x.bin", b"<?xml version='1.0'?><svg></svg>"), Kind::Svg);
        assert_eq!(sniff("x.png", b"\x89PNG"), Kind::Raster);
    }

    #[test]
    fn raster_is_scaled_down_and_opaque_has_no_hull() {
        let img = RgbaImage::from_pixel(400, 200, image::Rgba([255, 0, 0, 255]));
        let d = decode("a.png", &png_bytes(&img), 100, false).unwrap();
        assert_eq!((d.width(), d.height()), (100, 50));
        assert!(d.hull.is_none());
    }

    #[test]
    fn transparent_png_gets_hull() {
        let mut img = RgbaImage::new(64, 64);
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, image::Rgba([0, 255, 0, 255]));
            }
        }
        let d = decode("a.png", &png_bytes(&img), 220, false).unwrap();
        let hull = d.hull.expect("hull");
        assert!(hull.iter().all(|p| p[0].abs() <= 0.26 && p[1].abs() <= 0.26));
    }

    #[test]
    fn svg_upscales_when_asked() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="10" fill="red"/></svg>"#;
        let small = decode("a.svg", svg, 200, false).unwrap();
        assert_eq!(small.height(), 20);
        let big = decode("a.svg", svg, 200, true).unwrap();
        assert_eq!(big.height(), 200);
        assert!(big.hull.is_some());
    }
}
