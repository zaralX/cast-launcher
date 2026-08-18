use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};

pub const SIZE: u32 = 64;
pub const LEGACY_HEIGHT: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SkinVariant {
    #[default]
    Classic,
    Slim,
}

impl SkinVariant {
    pub fn as_api(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Slim => "slim",
        }
    }

    pub fn from_model(model: Option<&str>) -> Self {
        match model {
            Some(model) if model.eq_ignore_ascii_case("slim") => Self::Slim,
            _ => Self::Classic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Texture {
    pub fn blank(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * 4) as usize
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = self.index(x, y);
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        let at = self.index(x, y);
        self.rgba[at..at + 4].copy_from_slice(&pixel);
    }

    fn alpha(&self, x: u32, y: u32) -> u8 {
        self.rgba[self.index(x, y) + 3]
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

pub fn decode(bytes: &[u8]) -> CommandResult<Texture> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder
        .read_info()
        .map_err(|e| CommandError::fs("Это не png").with_details(e.to_string()))?;

    let mut buffer = vec![0_u8; reader.output_buffer_size().unwrap_or(0)];

    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| CommandError::fs("Не удалось прочитать png").with_details(e.to_string()))?;

    buffer.truncate(info.buffer_size());

    let pixels = (info.width * info.height) as usize;

    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => expand(&buffer, pixels, |chunk, out| {
            out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }, 3),
        png::ColorType::GrayscaleAlpha => expand(&buffer, pixels, |chunk, out| {
            out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
        }, 2),
        png::ColorType::Grayscale => expand(&buffer, pixels, |chunk, out| {
            out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], 255]);
        }, 1),
        png::ColorType::Indexed => {
            return Err(CommandError::fs("png с палитрой не поддерживается"))
        }
    };

    if rgba.len() != pixels * 4 {
        return Err(CommandError::fs("png повреждён"));
    }

    Ok(Texture {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn expand(
    buffer: &[u8],
    pixels: usize,
    convert: impl Fn(&[u8], &mut Vec<u8>),
    stride: usize,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels * 4);

    for chunk in buffer.chunks_exact(stride) {
        convert(chunk, &mut out);
    }

    out
}

pub fn encode(texture: &Texture) -> CommandResult<Vec<u8>> {
    let mut out = Vec::new();

    {
        let mut encoder = png::Encoder::new(&mut out, texture.width, texture.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| CommandError::fs("Не удалось записать png").with_details(e.to_string()))?;

        writer
            .write_image_data(&texture.rgba)
            .map_err(|e| CommandError::fs("Не удалось записать png").with_details(e.to_string()))?;
    }

    Ok(out)
}

pub fn normalize(bytes: &[u8]) -> CommandResult<(Vec<u8>, SkinVariant)> {
    let texture = decode(bytes)?;

    let texture = match (texture.width, texture.height) {
        (SIZE, SIZE) => texture,
        (SIZE, LEGACY_HEIGHT) => expand_legacy(&texture),
        (width, height) => {
            return Err(CommandError::fs(format!(
                "Скин должен быть 64x64 или 64x32, а этот {width}x{height}"
            )))
        }
    };

    let variant = detect_variant(&texture);

    Ok((encode(&texture)?, variant))
}

const SLIM_GAPS: [Rect; 4] = [
    Rect { x: 50, y: 16, w: 2, h: 4 },  // правая рука, верх и низ
    Rect { x: 54, y: 20, w: 2, h: 12 }, // правая рука, боковины
    Rect { x: 42, y: 48, w: 2, h: 4 },  // левая рука, верх и низ
    Rect { x: 46, y: 52, w: 2, h: 12 }, // левая рука, боковины
];

pub fn detect_variant(texture: &Texture) -> SkinVariant {
    if texture.width != SIZE || texture.height != SIZE {
        return SkinVariant::Classic;
    }

    let transparent = SLIM_GAPS.iter().all(|gap| {
        (gap.y..gap.y + gap.h).all(|y| (gap.x..gap.x + gap.w).all(|x| texture.alpha(x, y) == 0))
    });

    if transparent {
        SkinVariant::Slim
    } else {
        SkinVariant::Classic
    }
}

pub fn expand_legacy(legacy: &Texture) -> Texture {
    let mut texture = Texture::blank(SIZE, SIZE);

    for y in 0..legacy.height.min(LEGACY_HEIGHT) {
        for x in 0..legacy.width.min(SIZE) {
            texture.set_pixel(x, y, legacy.pixel(x, y));
        }
    }

    mirror_limb(&mut texture, (0, 16), (16, 48)); // нога
    mirror_limb(&mut texture, (40, 16), (32, 48)); // рука

    texture
}

fn limb_faces(u: u32, v: u32) -> [(Rect, Rect); 6] {
    let top = Rect { x: u + 4, y: v, w: 4, h: 4 };
    let bottom = Rect { x: u + 8, y: v, w: 4, h: 4 };
    let right = Rect { x: u, y: v + 4, w: 4, h: 12 };
    let front = Rect { x: u + 4, y: v + 4, w: 4, h: 12 };
    let left = Rect { x: u + 8, y: v + 4, w: 4, h: 12 };
    let back = Rect { x: u + 12, y: v + 4, w: 4, h: 12 };

    [
        (top, top),
        (bottom, bottom),
        (left, right),
        (front, front),
        (right, left),
        (back, back),
    ]
}

fn mirror_limb(texture: &mut Texture, from: (u32, u32), to: (u32, u32)) {
    let source = limb_faces(from.0, from.1);
    let target = limb_faces(to.0, to.1);

    for (index, (src, _)) in source.iter().enumerate() {
        let (_, dst) = target[index];

        for y in 0..src.h {
            for x in 0..src.w {
                let pixel = texture.pixel(src.x + x, src.y + y);
                texture.set_pixel(dst.x + dst.w - 1 - x, dst.y + y, pixel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_of(width: u32, height: u32, paint: impl Fn(&mut Texture)) -> Vec<u8> {
        let mut texture = Texture::blank(width, height);

        for y in 0..height {
            for x in 0..width {
                texture.set_pixel(x, y, [10, 20, 30, 255]);
            }
        }

        paint(&mut texture);
        encode(&texture).unwrap()
    }

    fn clear(texture: &mut Texture, rect: Rect) {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                texture.set_pixel(x, y, [0, 0, 0, 0]);
            }
        }
    }

    #[test]
    fn round_trip_keeps_every_pixel() {
        let bytes = png_of(64, 64, |texture| {
            texture.set_pixel(3, 7, [1, 2, 3, 4]);
        });

        let decoded = decode(&bytes).unwrap();

        assert_eq!(decoded.width, 64);
        assert_eq!(decoded.height, 64);
        assert_eq!(decoded.pixel(3, 7), [1, 2, 3, 4]);
    }

    #[test]
    fn classic_skin_is_detected_when_the_gaps_are_painted() {
        let bytes = png_of(64, 64, |_| {});
        let (_, variant) = normalize(&bytes).unwrap();

        assert_eq!(variant, SkinVariant::Classic);
    }

    #[test]
    fn slim_skin_is_detected_when_every_gap_is_transparent() {
        let bytes = png_of(64, 64, |texture| {
            for gap in SLIM_GAPS {
                clear(texture, gap);
            }
        });

        let (_, variant) = normalize(&bytes).unwrap();

        assert_eq!(variant, SkinVariant::Slim);
    }

    #[test]
    fn one_painted_gap_is_enough_to_stay_classic() {
        let bytes = png_of(64, 64, |texture| {
            for gap in SLIM_GAPS.iter().skip(1) {
                clear(texture, *gap);
            }
        });

        let (_, variant) = normalize(&bytes).unwrap();

        assert_eq!(variant, SkinVariant::Classic);
    }

    #[test]
    fn legacy_skins_grow_to_the_modern_layout() {
        let bytes = png_of(64, 32, |texture| {
            texture.set_pixel(44, 20, [200, 0, 0, 255]);
            texture.set_pixel(0, 20, [0, 200, 0, 255]);
        });

        let (normalized, _) = normalize(&bytes).unwrap();
        let texture = decode(&normalized).unwrap();

        assert_eq!(texture.height, 64);

        assert_eq!(texture.pixel(32 + 4 + 3, 52), [200, 0, 0, 255]);

        assert_eq!(texture.pixel(16 + 8 + 3, 52), [0, 200, 0, 255]);
    }

    #[test]
    fn legacy_expansion_keeps_the_original_half_intact() {
        let bytes = png_of(64, 32, |texture| {
            texture.set_pixel(10, 10, [7, 7, 7, 255]);
        });

        let (normalized, _) = normalize(&bytes).unwrap();
        let texture = decode(&normalized).unwrap();

        assert_eq!(texture.pixel(10, 10), [7, 7, 7, 255]);
    }

    #[test]
    fn wrong_sizes_are_rejected() {
        let error = normalize(&png_of(32, 32, |_| {})).unwrap_err();
        assert!(error.message.contains("64x64"));

        assert!(normalize(&png_of(128, 128, |_| {})).is_err());
        assert!(normalize(b"not a png").is_err());
    }

    #[test]
    fn variants_map_to_the_mojang_spelling() {
        assert_eq!(SkinVariant::Classic.as_api(), "classic");
        assert_eq!(SkinVariant::Slim.as_api(), "slim");
        assert_eq!(SkinVariant::from_model(Some("slim")), SkinVariant::Slim);
        assert_eq!(SkinVariant::from_model(Some("SLIM")), SkinVariant::Slim);
        assert_eq!(SkinVariant::from_model(None), SkinVariant::Classic);
    }
}
