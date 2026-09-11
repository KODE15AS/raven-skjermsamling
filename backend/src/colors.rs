/// Fast fargepalett med god kontrast på mørk bakgrunn og 70"-avstand.
pub const PALETTE: &[(&str, &str)] = &[
    ("gul", "#ffd400"),
    ("grønn", "#2ecc40"),
    ("blå", "#339cff"),
    ("rød", "#ff4136"),
    ("oransje", "#ff851b"),
    ("lilla", "#c46bff"),
    ("turkis", "#39cccc"),
    ("rosa", "#f012be"),
];

/// Velg en farge som ikke er i bruk. Foretrekk `preferred` (fra tidligere
/// session med samme navn) hvis den er ledig, ellers første ledige, ellers
/// gjenbruk preferred/første (flere enn 8 samtidige deltagere).
pub fn pick_color(preferred: Option<&str>, in_use: &[String]) -> String {
    if let Some(p) = preferred {
        if !in_use.iter().any(|c| c == p) {
            return p.to_string();
        }
    }
    for (_, hex) in PALETTE {
        if !in_use.iter().any(|c| c == hex) {
            return hex.to_string();
        }
    }
    preferred
        .map(|p| p.to_string())
        .unwrap_or_else(|| PALETTE[0].1.to_string())
}
