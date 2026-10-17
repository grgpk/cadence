//! The picture a shared booking link shows in a chat app or an email:
//! the host's name, colour and kinds of call, drawn as SVG and turned into
//! the PNG those apps need.

use std::sync::{Arc, LazyLock};

use resvg::{tiny_skia, usvg};

use super::models::HostPage;

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;
const INK: &str = "#111111";
const MUTED: &str = "#5c5c66";
const NAME_CHARS: usize = 22;
const TITLE_CHARS: usize = 38;
const TYPES_SHOWN: usize = 3;

const REGULAR: &[u8] = include_bytes!("fonts/HankenGrotesk-Regular.ttf");
const BOLD: &[u8] = include_bytes!("fonts/HankenGrotesk-Bold.ttf");

/// The site's own typeface, loaded once.
static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut fonts = usvg::fontdb::Database::new();
    fonts.load_font_data(REGULAR.to_vec());
    fonts.load_font_data(BOLD.to_vec());
    Arc::new(fonts)
});

/// What the card shows.
pub struct PreviewCard {
    pub host_name: String,
    pub accent: String,
    /// Title and length of each kind of call, in the page's order.
    pub types: Vec<(String, Option<i32>)>,
    /// The link as people read it, without `https://`.
    pub link: String,
    pub show_branding: bool,
}

impl PreviewCard {
    pub fn of(page: &HostPage, link: String) -> Self {
        Self {
            host_name: page.host_name.clone(),
            accent: page.accent_color.clone().unwrap_or_else(|| INK.to_owned()),
            types: page
                .types
                .iter()
                .map(|kind| (kind.title.clone(), kind.duration_minutes))
                .collect(),
            link,
            show_branding: page.show_branding,
        }
    }
}

/// The card as PNG bytes. `None` only if rendering fails, which a card
/// built here never should.
pub fn render(card: &PreviewCard) -> Option<Vec<u8>> {
    let options = usvg::Options {
        font_family: "Hanken Grotesk".to_owned(),
        fontdb: Arc::clone(&FONTS),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(&svg(card), &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT)?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

/// The card as SVG. Every text from the host is escaped and cut to fit:
/// SVG does not wrap.
pub fn svg(card: &PreviewCard) -> String {
    let mut rows: Vec<String> = card
        .types
        .iter()
        .take(TYPES_SHOWN)
        .enumerate()
        .map(|(index, (title, minutes))| {
            let y = 345 + 54 * index;
            let length = minutes.map(length_label).unwrap_or_default();
            format!(
                r#"<text x="120" y="{y}" font-size="38" fill="{INK}">{}</text><text x="1080" y="{y}" font-size="34" fill="{MUTED}" text-anchor="end">{length}</text>"#,
                escape(&cut(title, TITLE_CHARS)),
            )
        })
        .collect();
    let more = card.types.len().saturating_sub(TYPES_SHOWN);
    if more > 0 {
        let y = 345 + 54 * TYPES_SHOWN;
        rows.push(format!(
            r#"<text x="120" y="{y}" font-size="32" fill="{MUTED}">+{more} more</text>"#
        ));
    }
    let rows = rows.concat();
    let brand = if card.show_branding {
        format!(
            r#"<text x="1080" y="575" font-size="36" font-weight="700" fill="{INK}" text-anchor="end">Cadence</text>"#
        )
    } else {
        String::new()
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}"><rect width="{WIDTH}" height="{HEIGHT}" fill="#ffffff"/><rect width="28" height="{HEIGHT}" fill="{accent}"/><text x="120" y="150" font-size="40" fill="{MUTED}">Book a call with</text><text x="120" y="250" font-size="88" font-weight="700" fill="{INK}">{name}</text>{rows}<text x="120" y="575" font-size="30" fill="{MUTED}">{link}</text>{brand}</svg>"##,
        accent = escape(&card.accent),
        name = escape(&cut(&card.host_name, NAME_CHARS)),
        link = escape(&cut(&card.link, 48)),
    )
}

/// "30 min", "1 h", "1 h 30 min", as the booking page says it.
fn length_label(minutes: i32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, rest) => format!("{rest} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, rest) => format!("{hours} h {rest} min"),
    }
}

/// At most `limit` characters, ending in "…" when cut.
fn cut(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let kept: String = text.chars().take(limit.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> PreviewCard {
        PreviewCard {
            host_name: name.to_owned(),
            accent: "#7c3aed".to_owned(),
            types: vec![
                ("Intro call".to_owned(), Some(30)),
                ("Deep dive".to_owned(), Some(90)),
                ("Review".to_owned(), None),
                ("Office hours".to_owned(), Some(60)),
            ],
            link: "bookcadence.app/book/nino".to_owned(),
            show_branding: true,
        }
    }

    #[test]
    fn a_hosts_words_cannot_break_out_of_the_picture() {
        let drawn = svg(&card("<script>alert(1)</script> & co"));
        assert!(!drawn.contains("<script>"));
        assert!(drawn.contains("&lt;script&gt;"));
    }

    #[test]
    fn long_names_are_cut_and_extra_calls_counted() {
        let drawn = svg(&card("Bartholomew Montgomery-Fitzwilliam"));
        assert!(drawn.contains("Bartholomew Montgomer…"));
        assert!(drawn.contains("1 h 30 min"));
        assert!(drawn.contains("+1 more"));
        assert!(!drawn.contains("Office hours"));
    }

    /// Writes a Free and a Pro card as PNG files to look at:
    /// `CADENCE_PREVIEW_DIR=/some/dir cargo test -p cadence-backend --lib link_previews -- --ignored`
    #[test]
    #[ignore = "writes files; needs CADENCE_PREVIEW_DIR"]
    fn link_previews() {
        let directory = std::path::PathBuf::from(
            std::env::var("CADENCE_PREVIEW_DIR").expect("CADENCE_PREVIEW_DIR is set"),
        );
        std::fs::create_dir_all(&directory).expect("preview directory can be created");
        let mut free = card("Nino Tbilisi");
        free.accent = INK.to_owned();
        free.types.truncate(1);
        let mut pro = card("Mia Chen");
        pro.show_branding = false;
        for (name, card) in [("free", free), ("pro", pro)] {
            let png = render(&card).expect("a picture");
            std::fs::write(directory.join(format!("{name}.png")), png).expect("written");
        }
    }

    #[test]
    fn the_card_renders_as_a_png() {
        let png = render(&card("Nino Tbilisi")).expect("a picture");
        assert!(png.starts_with(b"\x89PNG"));
    }
}
