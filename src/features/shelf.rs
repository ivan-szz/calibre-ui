//! Shelves: named lists of library books, stored in the app's own database because the
//! Calibre library is only read.

use topcoat::{Result, context::Cx};

use crate::db::db;

/// A named list of books.
#[derive(Debug, toasty::Model)]
pub struct Shelf {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    /// The owner; each user sees only their own shelves.
    #[index]
    pub user_id: u64,

    #[has_many]
    pub entries: toasty::Deferred<Vec<ShelfEntry>>,
}

/// One book on a shelf. `book_id` is the Calibre book id.
#[derive(Debug, toasty::Model)]
pub struct ShelfEntry {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub shelf_id: u64,

    #[belongs_to]
    pub shelf: toasty::Deferred<Shelf>,

    #[index]
    pub book_id: u64,
}

/// A book a user has marked as read.
#[derive(Debug, toasty::Model)]
pub struct ReadMark {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[index]
    pub book_id: u64,
}

impl ReadMark {
    /// The ids of the books the user has read.
    pub async fn book_ids(cx: &Cx, user_id: u64) -> Result<Vec<u64>> {
        let marks = ReadMark::filter_by_user_id(user_id).exec(&mut db(cx)).await?;
        Ok(marks.into_iter().map(|m| m.book_id).collect())
    }

    /// Marks a book read, or unread if it already is.
    pub async fn toggle(cx: &Cx, user_id: u64, book_id: u64) -> Result<()> {
        let mut db = db(cx);
        let marked = ReadMark::filter_by_user_id(user_id)
            .filter(ReadMark::fields().book_id().eq(book_id))
            .first()
            .exec(&mut db)
            .await?
            .is_some();
        if marked {
            ReadMark::filter_by_user_id(user_id)
                .filter(ReadMark::fields().book_id().eq(book_id))
                .delete()
                .exec(&mut db)
                .await?;
        } else {
            toasty::create!(ReadMark { user_id, book_id }).exec(&mut db).await?;
        }
        Ok(())
    }
}

impl Shelf {
    /// A user's shelves, by name.
    pub async fn list(cx: &Cx, user_id: u64) -> Result<Vec<Shelf>> {
        Ok(Shelf::filter_by_user_id(user_id)
            .order_by(Shelf::fields().name().asc())
            .exec(&mut db(cx))
            .await?)
    }

    /// A shelf, if it exists and belongs to the user.
    pub async fn get(cx: &Cx, id: u64, user_id: u64) -> Result<Option<Shelf>> {
        let shelf = Shelf::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(shelf.filter(|s| s.user_id == user_id))
    }

    pub async fn add(cx: &Cx, name: &str, user_id: u64) -> Result<()> {
        toasty::create!(Shelf { name, user_id }).exec(&mut db(cx)).await?;
        Ok(())
    }

    /// Takes a book off every shelf, after it is deleted from the library.
    pub async fn forget_book(cx: &Cx, book_id: u64) -> Result<()> {
        ShelfEntry::filter_by_book_id(book_id).delete().exec(&mut db(cx)).await?;
        Ok(())
    }

    /// Removes a shelf and its entries. The books stay in the library.
    pub async fn remove(cx: &Cx, id: u64) -> Result<()> {
        let mut db = db(cx);
        ShelfEntry::filter_by_shelf_id(id).delete().exec(&mut db).await?;
        Shelf::filter_by_id(id).delete().exec(&mut db).await?;
        Ok(())
    }

    /// The ids of the books on a shelf, in the order they were added.
    pub async fn book_ids(cx: &Cx, id: u64) -> Result<Vec<u64>> {
        let entries = ShelfEntry::filter_by_shelf_id(id)
            .order_by(ShelfEntry::fields().id().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(entries.into_iter().map(|e| e.book_id).collect())
    }

    /// The ids of the shelves a book is on.
    pub async fn shelves_of(cx: &Cx, book_id: u64) -> Result<Vec<u64>> {
        let entries = ShelfEntry::filter_by_book_id(book_id).exec(&mut db(cx)).await?;
        Ok(entries.into_iter().map(|e| e.shelf_id).collect())
    }

    /// Puts a book on a shelf, or takes it off if it is already there.
    pub async fn toggle(cx: &Cx, id: u64, book_id: u64) -> Result<()> {
        let mut db = db(cx);
        let on_shelf = ShelfEntry::filter_by_book_id(book_id)
            .exec(&mut db)
            .await?
            .into_iter()
            .any(|e| e.shelf_id == id);
        if on_shelf {
            ShelfEntry::filter_by_book_id(book_id)
                .filter(ShelfEntry::fields().shelf_id().eq(id))
                .delete()
                .exec(&mut db)
                .await?;
        } else {
            toasty::create!(ShelfEntry { shelf_id: id, book_id }).exec(&mut db).await?;
        }
        Ok(())
    }
}
