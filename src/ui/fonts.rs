//! Runtime font selection, shared by both built-in egui families.
//!
//! Installed Neo XiHei is preferred for every character. The system's generic
//! sans face and other regular faces provide per-character Unicode fallbacks;
//! egui's defaults remain last. Discovery and cmap scanning happen once before
//! the first frame. No XiHei font is bundled, downloaded, or required to start.

use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions};

pub fn definitions() -> FontDefinitions {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    definitions_from_db(&db)
}

fn definitions_from_db(db: &fontdb::Database) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut coverage = Coverage::default();
    let mut selected = Vec::new();
    let mut names = Vec::new();

    // Family metadata includes localized aliases. Prefer Plus, then the base
    // family, and choose the closest available face to regular upright.
    let mut preferred: Vec<_> = db
        .faces()
        .filter_map(|face| xihei_priority(face).map(|priority| (priority, face)))
        .collect();
    preferred.sort_by_key(|(priority, face)| (*priority, rank(face), &face.post_script_name));
    for (_, face) in preferred {
        if append_face(db, face.id, true, &mut fonts, &mut coverage, &mut names) {
            selected.push(face.id);
            break;
        }
    }

    if let Some(id) = db.query(&fontdb::Query {
        families: &[fontdb::Family::SansSerif],
        ..Default::default()
    }) && !selected.contains(&id)
        && append_face(db, id, true, &mut fonts, &mut coverage, &mut names)
    {
        selected.push(id);
    }

    // Scan each remaining regular face once, copying bytes only for faces
    // that add coverage. A script probe would miss less common characters.
    let mut candidates: Vec<_> = db
        .faces()
        .filter(|face| {
            face.weight == fontdb::Weight::NORMAL
                && face.style == fontdb::Style::Normal
                && face.stretch == fontdb::Stretch::Normal
                && !selected.contains(&face.id)
        })
        .collect();
    candidates.sort_by_key(|face| &face.post_script_name);
    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter_map(|face| {
            let cmap = db.with_face_data(face.id, Coverage::from_font)??;
            Some((face.id, cmap))
        })
        .collect();
    // Prefer broad coverage over many nearly identical CJK faces. Bound the
    // retained fonts so rare glyphs cannot consume unbounded startup memory.
    let mut extra_bytes = 0;
    for _ in 0..16 {
        let Some((position, added)) = candidates
            .iter()
            .enumerate()
            .map(|(position, (_, cmap))| (position, cmap.additional_count(&coverage)))
            .max_by_key(|(position, count)| (*count, std::cmp::Reverse(*position)))
        else {
            break;
        };
        if added == 0 {
            break;
        }
        let (id, _) = candidates.remove(position);
        let size = db.with_face_data(id, |bytes, _| bytes.len()).unwrap_or(0);
        if extra_bytes + size > 128 * 1024 * 1024 {
            continue;
        }
        if append_face(db, id, false, &mut fonts, &mut coverage, &mut names) {
            extra_bytes += size;
        }
    }

    // Both proportional and monospace text prefer XiHei, including Latin.
    // Register each face only once; the family chains share its FontData.
    for chain in fonts.families.values_mut() {
        chain.retain(|name| !names.contains(name));
        chain.splice(0..0, names.iter().cloned());
    }
    fonts
}

fn xihei_priority(face: &fontdb::FaceInfo) -> Option<u8> {
    face.families
        .iter()
        .filter_map(|(name, _)| {
            let name: String = name
                .chars()
                .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '_'))
                .flat_map(char::to_lowercase)
                .map(|c| if c == '＋' { '+' } else { c })
                .collect();
            match name.as_str() {
                "lxgwneoxiheiplus"
                | "霞鹜新晰黑plus"
                | "霞鶩新晰黑plus"
                | "霞鹜新晰黑+"
                | "霞鶩新晰黑+" => Some(0),
                "lxgwneoxihei" | "霞鹜新晰黑" | "霞鶩新晰黑" => Some(1),
                _ => None,
            }
        })
        .min()
}

fn rank(face: &fontdb::FaceInfo) -> (u8, u16, bool) {
    (
        u8::from(face.style != fontdb::Style::Normal),
        face.weight.0.abs_diff(400),
        face.stretch != fontdb::Stretch::Normal,
    )
}

/// Append a readable face, retaining the collection index. Preferred and generic
/// faces keep their priority even when an earlier font covers the same glyphs.
fn append_face(
    db: &fontdb::Database,
    id: fontdb::ID,
    primary: bool,
    fonts: &mut FontDefinitions,
    covered: &mut Coverage,
    names: &mut Vec<String>,
) -> bool {
    db.with_face_data(id, |bytes, index| {
        let Some(coverage) = Coverage::from_font(bytes, index) else {
            return false;
        };
        if !primary && !coverage.adds_to(covered) {
            return false;
        }

        // Reuse identical data already present (including egui defaults).
        // This also avoids copying duplicate installations of the same face.
        let existing = fonts
            .font_data
            .iter()
            .find(|(_, data)| data.index == index && data.font.as_ref() == bytes)
            .map(|(name, _)| name.clone());
        let name = existing.unwrap_or_else(|| {
            let name = format!("system-font-{}", fonts.font_data.len());
            let mut data = FontData::from_owned(bytes.to_vec());
            data.index = index;
            fonts.font_data.insert(name.clone(), Arc::new(data));
            name
        });
        if !names.contains(&name) {
            names.push(name);
        }
        covered.extend(&coverage);
        true
    })
    .unwrap_or(false)
}

/// A small fixed-size Unicode bitset (136 KiB), independent of glyph count.
/// It avoids allocating a tree or hash entry for every CJK character.
struct Coverage(Vec<u64>);

impl Default for Coverage {
    fn default() -> Self {
        Self(vec![0; 0x110000 / 64])
    }
}

impl Coverage {
    fn from_font(bytes: &[u8], index: u32) -> Option<Self> {
        let face = ttf_parser::Face::parse(bytes, index).ok()?;
        let mut coverage = Self::default();
        // Unicode cmaps commonly repeat the same mapping for multiple
        // platforms. Prefer a full-repertoire table; otherwise use the first
        // Unicode table, as ttf-parser does for character lookup.
        let subtable = face
            .tables()
            .cmap?
            .subtables
            .into_iter()
            .filter(|table| table.is_unicode())
            .max_by_key(|table| {
                matches!(table.format, ttf_parser::cmap::Format::SegmentedCoverage(_))
            })?;
        // Format 12 maps sequential ranges. Walking the raw groups avoids a
        // binary search per glyph in large CJK fonts while excluding glyph 0.
        if matches!(
            subtable.format,
            ttf_parser::cmap::Format::SegmentedCoverage(_)
        ) {
            let cmap = face
                .raw_face()
                .table(ttf_parser::Tag::from_bytes(b"cmap"))?;
            let count = u16::from_be_bytes(cmap.get(2..4)?.try_into().ok()?) as usize;
            for record in cmap.get(4..4 + count * 8)?.as_chunks::<8>().0 {
                let platform = u16::from_be_bytes(record[0..2].try_into().ok()?);
                let encoding = u16::from_be_bytes(record[2..4].try_into().ok()?);
                if platform != subtable.platform_id as u16 || encoding != subtable.encoding_id {
                    continue;
                }
                let offset = u32::from_be_bytes(record[4..8].try_into().ok()?) as usize;
                let table = cmap.get(offset..)?;
                let count = u32::from_be_bytes(table.get(12..16)?.try_into().ok()?) as usize;
                for group in table.get(16..16 + count * 12)?.as_chunks::<12>().0 {
                    let start = u32::from_be_bytes(group[0..4].try_into().ok()?);
                    let end = u32::from_be_bytes(group[4..8].try_into().ok()?).min(0x10ffff);
                    let glyph = u32::from_be_bytes(group[8..12].try_into().ok()?);
                    for codepoint in start..=end {
                        if char::from_u32(codepoint).is_some()
                            && glyph
                                .checked_add(codepoint - start)
                                .is_some_and(|g| g > 0 && g <= u16::MAX as u32)
                        {
                            coverage.0[codepoint as usize / 64] |= 1 << (codepoint % 64);
                        }
                    }
                }
                break;
            }
        } else {
            subtable.codepoints(|codepoint| {
                if char::from_u32(codepoint).is_some()
                    && subtable
                        .glyph_index(codepoint)
                        .is_some_and(|glyph| glyph.0 != 0)
                {
                    coverage.0[codepoint as usize / 64] |= 1 << (codepoint % 64);
                }
            });
        }
        Some(coverage)
    }

    fn adds_to(&self, covered: &Self) -> bool {
        self.0.iter().zip(&covered.0).any(|(a, b)| a & !b != 0)
    }

    fn additional_count(&self, covered: &Self) -> u32 {
        self.0
            .iter()
            .zip(&covered.0)
            .map(|(a, b)| (a & !b).count_ones())
            .sum()
    }

    fn extend(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            *a |= b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontFamily;
    use eframe::egui::epaint::text::TextOptions;
    use eframe::egui::text::Fonts;

    // Existing egui bytes stand in for installed fonts. Tests never depend on
    // which fonts happen to be installed on the developer or CI machine.
    fn install(db: &mut fontdb::Database, fixture: &str, family: &str) -> fontdb::ID {
        let data = &FontDefinitions::default().font_data[fixture];
        let mut source = fontdb::Database::new();
        source.load_font_data(data.font.to_vec());
        let mut face = source.faces().next().unwrap().clone();
        face.families[0].0 = family.to_owned();
        face.post_script_name = family.to_owned();
        face.weight = fontdb::Weight::NORMAL;
        face.style = fontdb::Style::Normal;
        face.stretch = fontdb::Stretch::Normal;
        db.push_face_info(face)
    }

    #[test]
    fn installed_font_discovery_smoke_test() {
        let start = std::time::Instant::now();
        let fonts = definitions();
        let bytes: usize = fonts.font_data.values().map(|data| data.font.len()).sum();
        eprintln!(
            "runtime fonts: {} faces, {:.1} MiB, {:?}",
            fonts.font_data.len(),
            bytes as f64 / (1024.0 * 1024.0),
            start.elapsed()
        );
        assert!(!fonts.families[&FontFamily::Proportional].is_empty());
    }

    #[test]
    fn no_installed_fonts_keeps_egui_defaults_and_can_draw() {
        let fonts = definitions_from_db(&fontdb::Database::new());
        assert_eq!(fonts.families, FontDefinitions::default().families);
        let mut drawer = Fonts::new(TextOptions::default(), fonts);
        assert!(drawer.has_glyphs(
            &eframe::egui::FontId::proportional(15.0),
            "Missing XiHei: ready!"
        ));
    }

    #[test]
    fn xihei_precedes_generic_sans_and_defaults_for_both_families() {
        let mut db = fontdb::Database::new();
        install(&mut db, "Hack", "System Sans");
        db.set_sans_serif_family("System Sans");
        install(&mut db, "Ubuntu-Light", "LXGW Neo XiHei Plus");
        let fonts = definitions_from_db(&db);
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            assert_eq!(&fonts.families[&family][..2], ["Ubuntu-Light", "Hack"]);
        }
        // Reuse the fixture data rather than copying it under a second name.
        assert_eq!(
            fonts.font_data.len(),
            FontDefinitions::default().font_data.len()
        );
    }

    #[test]
    fn absent_xihei_immediately_uses_generic_sans() {
        let mut db = fontdb::Database::new();
        install(&mut db, "Hack", "System Sans");
        db.set_sans_serif_family("System Sans");
        let fonts = definitions_from_db(&db);
        for chain in fonts.families.values() {
            assert_eq!(chain[0], "Hack");
        }
    }

    #[test]
    fn plus_and_localized_aliases_are_preferred_over_base_family() {
        for alias in [
            "LXGW Neo XiHei Plus",
            "霞鹜新晰黑 Plus",
            "霞鶩新晰黑 Plus",
            "霞鹜新晰黑＋",
            "霞鶩新晰黑＋",
        ] {
            let mut db = fontdb::Database::new();
            install(&mut db, "Hack", "LXGW Neo XiHei");
            install(&mut db, "Ubuntu-Light", alias);
            let fonts = definitions_from_db(&db);
            assert_eq!(fonts.families[&FontFamily::Proportional][0], "Ubuntu-Light");
        }
        let mut db = fontdb::Database::new();
        install(&mut db, "Hack", "霞鹜新晰黑");
        assert_eq!(
            definitions_from_db(&db).families[&FontFamily::Proportional][0],
            "Hack"
        );
    }

    #[test]
    fn only_regular_faces_with_additional_unicode_coverage_are_added() {
        let mut db = fontdb::Database::new();
        install(&mut db, "Hack", "A Sans");
        db.set_sans_serif_family("A Sans");
        install(&mut db, "Hack", "B Duplicate");
        let italic = install(&mut db, "NotoEmoji-Regular", "C Italic");
        let mut face = db.face(italic).unwrap().clone();
        db.remove_face(italic);
        face.style = fontdb::Style::Italic;
        db.push_face_info(face);
        install(&mut db, "NotoEmoji-Regular", "D Extra Coverage");
        let fonts = definitions_from_db(&db);
        let chain = &fonts.families[&FontFamily::Proportional];
        assert_eq!(&chain[..2], ["Hack", "NotoEmoji-Regular"]);
        assert_eq!(
            chain,
            &[
                "Hack",
                "NotoEmoji-Regular",
                "Ubuntu-Light",
                "emoji-icon-font"
            ]
        );
    }

    #[test]
    fn collection_face_index_is_preserved() {
        let defaults = FontDefinitions::default();
        let sfnt = defaults.font_data["Hack"].font.as_ref();
        // Build a two-face collection without committing a font fixture.
        // TTC table offsets are file-relative, including in the second face.
        let mut collection = b"ttcf\0\x01\0\0\0\0\0\x02\0\0\0\0\0\0\0\0".to_vec();
        let table_count = u16::from_be_bytes(sfnt[4..6].try_into().unwrap()) as usize;
        for index in 0..2 {
            let start = collection.len();
            collection[12 + index * 4..16 + index * 4]
                .copy_from_slice(&(start as u32).to_be_bytes());
            collection.extend_from_slice(sfnt);
            for table in 0..table_count {
                let offset = start + 12 + table * 16 + 8;
                let value = u32::from_be_bytes(collection[offset..offset + 4].try_into().unwrap());
                collection[offset..offset + 4]
                    .copy_from_slice(&(value + start as u32).to_be_bytes());
            }
            while !collection.len().is_multiple_of(4) {
                collection.push(0);
            }
        }
        let mut db = fontdb::Database::new();
        let id = install(&mut db, "Hack", "LXGW Neo XiHei");
        let mut face = db.face(id).unwrap().clone();
        db.remove_face(id);
        face.source = fontdb::Source::Binary(Arc::new(collection));
        face.index = 1;
        db.push_face_info(face);
        let fonts = definitions_from_db(&db);
        let name = &fonts.families[&FontFamily::Proportional][0];
        assert_eq!(fonts.font_data[name].index, 1);
        let data = &fonts.font_data[name];
        let parsed = ttf_parser::Face::parse(data.font.as_ref(), data.index).unwrap();
        assert!(
            "Collection"
                .chars()
                .all(|c| parsed.glyph_index(c).is_some())
        );
    }

    #[test]
    fn unreadable_preferred_face_does_not_block_system_fallback() {
        let mut db = fontdb::Database::new();
        let id = install(&mut db, "Ubuntu-Light", "LXGW Neo XiHei Plus");
        let mut face = db.face(id).unwrap().clone();
        db.remove_face(id);
        face.source = fontdb::Source::Binary(Arc::new(vec![0; 16]));
        db.push_face_info(face);
        install(&mut db, "Hack", "System Sans");
        db.set_sans_serif_family("System Sans");
        assert_eq!(
            definitions_from_db(&db).families[&FontFamily::Proportional][0],
            "Hack"
        );
    }
}
