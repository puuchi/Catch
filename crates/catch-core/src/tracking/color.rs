use super::*;
const PALETTE: [&str; 8] = [
    "#2679e8", "#d58922", "#a24dc1", "#26965a", "#cd4258", "#219aa7", "#a9a12a", "#78523a",
];
pub(super) fn valid(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}
fn lab(hex: &str) -> [f64; 3] {
    let rgb: [f64; 3] = std::array::from_fn(|i| {
        let value = u8::from_str_radix(&hex[1 + i * 2..3 + i * 2], 16).unwrap() as f64 / 255.;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    let xyz = [
        (rgb[0] * 0.4124 + rgb[1] * 0.3576 + rgb[2] * 0.1805) / 0.95047,
        rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722,
        (rgb[0] * 0.0193 + rgb[1] * 0.1192 + rgb[2] * 0.9505) / 1.08883,
    ]
    .map(|value| {
        if value > 0.008856 {
            value.cbrt()
        } else {
            7.787 * value + 16. / 116.
        }
    });
    [
        116. * xyz[1] - 16.,
        500. * (xyz[0] - xyz[1]),
        200. * (xyz[1] - xyz[2]),
    ]
}
pub(super) fn next_color(doc: &Document) -> String {
    let colors: Vec<String> = doc
        .activities
        .iter()
        .chain(doc.trash.iter().map(|entry| &entry.activity))
        .filter_map(|matter| matter.color.as_deref())
        .filter(|color| valid(color))
        .map(str::to_lowercase)
        .collect();
    if colors.is_empty() {
        return PALETTE[0].into();
    }
    let counts = PALETTE.map(|color| {
        colors
            .iter()
            .filter(|existing| existing.as_str() == color)
            .count()
    });
    let minimum = *counts.iter().min().unwrap();
    let mut best = (PALETTE[0], -1_f64);
    for (index, color) in PALETTE.iter().enumerate() {
        if counts[index] != minimum {
            continue;
        }
        let a = lab(color);
        let separation = colors
            .iter()
            .filter(|other| other.as_str() != *color)
            .map(|other| {
                let b = lab(other);
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
            })
            .reduce(f64::min)
            .unwrap_or(0.);
        if separation > best.1 {
            best = (color, separation);
        }
    }
    best.0.into()
}
