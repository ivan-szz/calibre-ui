CREATE TABLE "reading_positions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "book_id" INTEGER NOT NULL,
    "format" TEXT NOT NULL,
    "position" TEXT NOT NULL,
    "percent" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_reading_positions_by_user_id" ON "reading_positions" ("user_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_reading_positions_by_book_id" ON "reading_positions" ("book_id");
-- #[toasty::breakpoint]
CREATE TABLE "reader_prefs" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "theme" TEXT NOT NULL,
    "font" TEXT NOT NULL,
    "size" INTEGER NOT NULL,
    "line_height" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_reader_prefs_by_user_id" ON "reader_prefs" ("user_id");
