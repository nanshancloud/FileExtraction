use std::io::Cursor;

/// Decoded application logo (RGBA8 pixels, row-major, top-left origin)
pub struct Logo {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Embedded logo file (256x256 PNG)
const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");

/// Decode the embedded logo into RGBA8 pixels.
/// Returns None when the asset is missing or cannot be decoded.
pub fn load_logo() -> Option<Logo> {
    let mut decoder = png::Decoder::new(Cursor::new(LOGO_PNG));
    // Expand palette/low-bit-depth and convert 16-bit samples down to 8-bit
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());

    let rgba = to_rgba8(&buf, info.color_type)?;
    Some(Logo {
        rgba,
        width: info.width,
        height: info.height,
    })
}

/// Normalize decoded PNG samples to RGBA8 regardless of the source color type
fn to_rgba8(data: &[u8], color_type: png::ColorType) -> Option<Vec<u8>> {
    let out = match color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        // Indexed images are expanded by the decoder; treat as unsupported here
        png::ColorType::Indexed => return None,
    };
    Some(out)
}
