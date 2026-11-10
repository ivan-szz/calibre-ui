//! The in-browser reader's state, kept on the account so it follows the user to any
//! browser: where they are in each book, and how they like the text shown.

use topcoat::{Result, context::Cx};

use crate::db::db;

/// Where a user is in one format of a book: an EPUB CFI, or a comic's page number.
#[derive(Debug, toasty::Model)]
pub struct ReadingPosition {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[index]
    pub book_id: u64,

    /// The upper-case format, like `EPUB`.
    pub format: String,

    pub position: String,

    /// How far through the book, 0-100.
    pub percent: u64,
}

impl ReadingPosition {
    pub async fn get(cx: &Cx, user_id: u64, book_id: u64, format: &str) -> Result<Option<ReadingPosition>> {
        Ok(ReadingPosition::filter_by_user_id(user_id)
            .filter(ReadingPosition::fields().book_id().eq(book_id))
            .filter(ReadingPosition::fields().format().eq(format.to_owned()))
            .first()
            .exec(&mut db(cx))
            .await?)
    }

    /// The books the user has started and not finished, most recently read first, with
    /// the format and percent of their latest position. `save` replaces the row, so a
    /// higher id is a more recent position.
    pub async fn in_progress(cx: &Cx, user_id: u64) -> Result<Vec<ReadingPosition>> {
        Ok(latest_unfinished(ReadingPosition::filter_by_user_id(user_id).exec(&mut db(cx)).await?))
    }

    pub async fn save(cx: &Cx, user_id: u64, book_id: u64, format: &str, position: String, percent: u64) -> Result<()> {
        let mut db = db(cx);
        ReadingPosition::filter_by_user_id(user_id)
            .filter(ReadingPosition::fields().book_id().eq(book_id))
            .filter(ReadingPosition::fields().format().eq(format.to_owned()))
            .delete()
            .exec(&mut db)
            .await?;
        let percent = percent.min(100);
        toasty::create!(ReadingPosition { user_id, book_id, format: format.to_owned(), position, percent })
            .exec(&mut db)
            .await?;
        Ok(())
    }
}

/// The newest position of each book, newest first, keeping the books neither unstarted
/// nor finished. With several formats, the most recently read one decides.
fn latest_unfinished(mut positions: Vec<ReadingPosition>) -> Vec<ReadingPosition> {
    positions.sort_by(|a, b| b.id.cmp(&a.id));
    let mut seen = std::collections::HashSet::new();
    positions.retain(|p| seen.insert(p.book_id));
    positions.retain(|p| (1..100).contains(&p.percent));
    positions
}

#[cfg(test)]
mod in_progress_tests {
    use super::*;

    fn at(id: u64, book_id: u64, percent: u64) -> ReadingPosition {
        ReadingPosition { id, user_id: 1, book_id, format: "EPUB".into(), position: String::new(), percent }
    }

    #[test]
    fn keeps_latest_unfinished_newest_first() {
        let kept = latest_unfinished(vec![at(1, 10, 30), at(2, 20, 100), at(3, 30, 50), at(4, 10, 60), at(5, 40, 0)]);
        let got: Vec<(u64, u64)> = kept.iter().map(|p| (p.book_id, p.percent)).collect();
        // Book 10's newer 60% wins over its 30%; 20 is finished and 40 not started.
        assert_eq!(got, [(10, 60), (30, 50)]);
    }
}

/// The page colours the reader offers. Cream is the default: it tires the eyes less
/// than white.
pub const THEMES: [&str; 4] = ["cream", "sepia", "white", "dark"];

/// The typefaces the reader offers. `publisher` keeps the book's own.
pub const FONTS: [&str; 6] = ["publisher", "literata", "serif", "sans", "atkinson", "dyslexic"];

/// How a user likes the reader's text shown.
#[derive(Debug, toasty::Model)]
pub struct ReaderPrefs {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub user_id: u64,

    /// One of [`THEMES`].
    pub theme: String,

    /// One of [`FONTS`].
    pub font: String,

    /// Text size, in percent of the book's own.
    pub size: u64,

    /// Line height, in hundredths.
    pub line_height: u64,
}

/// A user's reader settings, as plain values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub theme: String,
    pub font: String,
    pub size: u64,
    pub line_height: u64,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { theme: "cream".into(), font: "literata".into(), size: 100, line_height: 160 }
    }
}

impl Prefs {
    /// Replaces unknown or out-of-range values with the defaults' or the nearest allowed.
    pub fn clean(self) -> Prefs {
        let default = Prefs::default();
        Prefs {
            theme: if THEMES.contains(&self.theme.as_str()) { self.theme } else { default.theme },
            font: if FONTS.contains(&self.font.as_str()) { self.font } else { default.font },
            size: self.size.clamp(70, 200),
            line_height: self.line_height.clamp(120, 220),
        }
    }

    pub async fn load(cx: &Cx, user_id: u64) -> Result<Prefs> {
        let stored = ReaderPrefs::filter_by_user_id(user_id).first().exec(&mut db(cx)).await?;
        Ok(stored.map_or_else(Prefs::default, |p| {
            Prefs { theme: p.theme, font: p.font, size: p.size, line_height: p.line_height }.clean()
        }))
    }

    pub async fn save(self, cx: &Cx, user_id: u64) -> Result<()> {
        let Prefs { theme, font, size, line_height } = self.clean();
        let mut db = db(cx);
        ReaderPrefs::filter_by_user_id(user_id).delete().exec(&mut db).await?;
        toasty::create!(ReaderPrefs { user_id, theme, font, size, line_height }).exec(&mut db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Prefs;

    #[test]
    fn cleans_prefs() {
        let wild = Prefs { theme: "neon".into(), font: "comic".into(), size: 999, line_height: 10 };
        assert_eq!(wild.clean(), Prefs { theme: "cream".into(), font: "literata".into(), size: 200, line_height: 120 });
        assert_eq!(Prefs::default().clean(), Prefs::default());
    }
}
