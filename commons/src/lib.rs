#[derive(Clone)]
pub struct BDFGlyph {
    pub startchar: String,
    pub encoding: usize,
    pub swidth: (usize, usize),
    pub dwidth: (usize, usize),
    pub bbx: (usize, usize, isize, isize),
    pub bitmap: Vec<u16>,
}

#[derive(Clone)]
pub struct BDFProperties {
    pub point_size: usize,
    pub pixel_size: usize,
    pub resolution_x: usize,
    pub resolution_y: usize,
    pub font_ascent: usize,
    pub font_descent: usize,
    pub average_width: usize,
    pub spacing: String,
    pub gbdfed_info: String,
    pub charset_encoding: String,
    pub charset_registry: String,
    pub family_name: String,
    pub foundry: String,
    pub setwidth_name: String,
    pub slant: String,
    pub weight_name: String,
}

#[derive(Clone)]
pub struct BDFFont {
    pub size: (usize, usize, usize),
    pub font: String,
    pub bounding_box: (usize, usize, isize, isize),
    pub charcount: usize,
    pub properties: BDFProperties,
    pub glyphs: Vec<BDFGlyph>,
}

pub type Curves = Vec<(String, usize, Vec<Vec<(f32, f32)>>)>;

pub fn load_bdf(contents: &str) -> Result<BDFFont, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = contents.lines().collect();
    let mut i = 0;

    let mut font = BDFFont {
        size: (0, 0, 0),
        font: String::new(),
        bounding_box: (0, 0, 0, 0),
        charcount: 0,
        properties: BDFProperties {
            point_size: 0,
            pixel_size: 0,
            resolution_x: 0,
            resolution_y: 0,
            font_ascent: 0,
            font_descent: 0,
            average_width: 0,
            spacing: String::new(),
            gbdfed_info: String::new(),
            charset_encoding: String::new(),
            charset_registry: String::new(),
            family_name: String::new(),
            foundry: String::new(),
            setwidth_name: String::new(),
            slant: String::new(),
            weight_name: String::new(),
        },
        glyphs: Vec::new(),
    };

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("STARTFONT ") {
            i += 1;
        } else if line.starts_with("FONT ") {
            font.font = line[5..].to_string();
            i += 1;
        } else if line.starts_with("SIZE ") {
            let parts: Vec<&str> = line[5..].split_whitespace().collect();
            font.size = (
                parts[0].parse::<usize>()?,
                parts[1].parse::<usize>()?,
                parts[2].parse::<usize>()?,
            );
            i += 1;
        } else if line.starts_with("FONTBOUNDINGBOX ") {
            let parts: Vec<&str> = line[16..].split_whitespace().collect();
            font.bounding_box = (
                parts[0].parse::<usize>()?,
                parts[1].parse::<usize>()?,
                parts[2].parse::<isize>()?,
                parts[3].parse::<isize>()?,
            );
            i += 1;
        } else if line.starts_with("STARTPROPERTIES ") {
            let _num_props: usize = line[16..].parse()?;
            i += 1;
            while i < lines.len() && !lines[i].starts_with("ENDPROPERTIES") {
                let prop_line = lines[i];
                if prop_line.starts_with("POINT_SIZE ") {
                    font.properties.point_size = prop_line[11..].parse::<usize>()?;
                } else if prop_line.starts_with("PIXEL_SIZE ") {
                    font.properties.pixel_size = prop_line[11..].parse::<usize>()?;
                } else if prop_line.starts_with("RESOLUTION_X ") {
                    font.properties.resolution_x = prop_line[13..].parse::<usize>()?;
                } else if prop_line.starts_with("RESOLUTION_Y ") {
                    font.properties.resolution_y = prop_line[13..].parse::<usize>()?;
                } else if prop_line.starts_with("FONT_ASCENT ") {
                    font.properties.font_ascent = prop_line[12..].parse::<usize>()?;
                } else if prop_line.starts_with("FONT_DESCENT ") {
                    font.properties.font_descent = prop_line[13..].parse::<usize>()?;
                } else if prop_line.starts_with("AVERAGE_WIDTH ") {
                    font.properties.average_width = prop_line[14..].parse::<usize>()?;
                } else if prop_line.starts_with("SPACING ") {
                    font.properties.spacing = prop_line[8..].to_string();
                } else if prop_line.starts_with("_GBDFED_INFO ") {
                    font.properties.gbdfed_info = prop_line[14..].to_string();
                } else if prop_line.starts_with("CHARSET_ENCODING ") {
                    font.properties.charset_encoding = prop_line[17..].to_string();
                } else if prop_line.starts_with("CHARSET_REGISTRY ") {
                    font.properties.charset_registry = prop_line[17..].to_string();
                } else if prop_line.starts_with("FAMILY_NAME ") {
                    font.properties.family_name = prop_line[12..].to_string();
                } else if prop_line.starts_with("FOUNDRY ") {
                    font.properties.foundry = prop_line[8..].to_string();
                } else if prop_line.starts_with("SETWIDTH_NAME ") {
                    font.properties.setwidth_name = prop_line[14..].to_string();
                } else if prop_line.starts_with("SLANT ") {
                    font.properties.slant = prop_line[6..].to_string();
                } else if prop_line.starts_with("WEIGHT_NAME ") {
                    font.properties.weight_name = prop_line[12..].to_string();
                }
                i += 1;
            }
            i += 1;
        } else if line.starts_with("CHARS ") {
            font.charcount = line[6..].parse::<usize>()?;
            i += 1;
        } else if line.starts_with("STARTCHAR ") {
            let mut glyph = BDFGlyph {
                startchar: line[10..].to_string(),
                encoding: 0,
                swidth: (0, 0),
                dwidth: (0, 0),
                bbx: (0, 0, 0, 0),
                bitmap: Vec::new(),
            };
            i += 1;
            while i < lines.len() {
                let line = lines[i];
                if line.starts_with("ENCODING ") {
                    glyph.encoding = line[9..].parse::<usize>()?;
                    i += 1;
                } else if line.starts_with("SWIDTH ") {
                    let parts: Vec<&str> = line[7..].split_whitespace().collect();
                    glyph.swidth = (parts[0].parse::<usize>()?, parts[1].parse::<usize>()?);
                    i += 1;
                } else if line.starts_with("DWIDTH ") {
                    let parts: Vec<&str> = line[7..].split_whitespace().collect();
                    glyph.dwidth = (parts[0].parse::<usize>()?, parts[1].parse::<usize>()?);
                    i += 1;
                } else if line.starts_with("BBX ") {
                    let parts: Vec<&str> = line[4..].split_whitespace().collect();
                    glyph.bbx = (
                        parts[0].parse::<usize>()?,
                        parts[1].parse::<usize>()?,
                        parts[2].parse::<isize>()?,
                        parts[3].parse::<isize>()?,
                    );
                    i += 1;
                } else if line == "BITMAP" {
                    i += 1;
                    while i < lines.len() && lines[i] != "ENDCHAR" {
                        let hex_str = lines[i];
                        let value = u16::from_str_radix(hex_str, 16)?;

                        let normalized_value = if hex_str.len() <= 2 {
                            value << 8
                        } else {
                            value
                        };
                        glyph.bitmap.push(normalized_value);
                        i += 1;
                    }
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            font.glyphs.push(glyph);
        } else if line == "ENDFONT" {
            break;
        } else {
            i += 1;
        }
    }

    Ok(font)
}

pub fn bdf_to_rects(glyph: &BDFGlyph) -> Vec<Vec<(f32, f32)>> {
    let mut rects = Vec::new();
    let (_, height, x_off, y_off) = glyph.bbx;

    for (row_idx, &row_data) in glyph.bitmap.iter().enumerate() {
        for col_idx in 0..glyph.bbx.0 {
            if (row_data >> (15 - col_idx)) & 1 != 0 {
                let x = (x_off + col_idx as isize) as f32;
                let y = (y_off + (height as isize - 1 - row_idx as isize)) as f32;

                rects.push(vec![
                    (x, y),
                    (x + 1.0, y),
                    (x + 1.0, y + 1.0),
                    (x, y + 1.0),
                    (x, y),
                ]);
            }
        }
    }
    rects
}

pub fn bdf_to_curves(bdf: &BDFFont) -> Vec<(String, usize, Vec<Vec<(f32, f32)>>)> {
    bdf.glyphs
        .iter()
        .map(|glyph| {
            let mut paths = Vec::new();
            let (bbx_width, bbx_height, x_offset, y_offset) = glyph.bbx;

            let font_ascent = bdf.properties.font_ascent as isize;

            for (row_idx, &row_bits) in glyph.bitmap.iter().enumerate() {
                let y = (font_ascent - y_offset - bbx_height as isize + row_idx as isize) as f32;

                let mut start_col: Option<usize> = None;

                for col_idx in 0..bbx_width {
                    let bit_is_set = (row_bits >> (15 - col_idx)) & 1 != 0;

                    match (bit_is_set, start_col) {
                        (true, None) => start_col = Some(col_idx),
                        (false, Some(start)) => {
                            let x1 = (x_offset + start as isize) as f32;
                            let x2 = (x_offset + col_idx as isize) as f32;
                            paths.push(vec![(x1, y), (x2, y)]);
                            start_col = None;
                        }
                        _ => {}
                    }
                }

                if let Some(start) = start_col {
                    let x1 = (x_offset + start as isize) as f32;
                    let x2 = (x_offset + bbx_width as isize) as f32;
                    paths.push(vec![(x1, y), (x2, y)]);
                }
            }

            (glyph.encoding.to_string(), glyph.dwidth.0, paths)
        })
        .collect()
}

fn create_line(start_pixel: usize, end_pixel: usize, x_offset: isize, y: f32) -> Vec<(f32, f32)> {
    let x1 = (x_offset + start_pixel as isize) as f32;
    let x2 = (x_offset + end_pixel as isize) as f32;
    vec![(x1, y), (x2, y)]
}

pub static HAXOR_FONT: &str = include_str!("HaxorNarrow-17.bdf");
