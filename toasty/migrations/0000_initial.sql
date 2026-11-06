CREATE TABLE "shelves" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "user_id" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_shelves_by_user_id" ON "shelves" ("user_id");
-- #[toasty::breakpoint]
CREATE TABLE "users" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "password_hash" TEXT NOT NULL,
    "ereader_email" TEXT NOT NULL,
    "hidden_tags" TEXT NOT NULL,
    "kobo_token" TEXT NOT NULL,
    "admin" BOOLEAN NOT NULL,
    "can_download" BOOLEAN NOT NULL,
    "can_upload" BOOLEAN NOT NULL,
    "can_edit" BOOLEAN NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_users_by_name" ON "users" ("name");
-- #[toasty::breakpoint]
CREATE INDEX "index_users_by_kobo_token" ON "users" ("kobo_token");
-- #[toasty::breakpoint]
CREATE TABLE "user_sessions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "token_hash" TEXT NOT NULL,
    "user_id" INTEGER NOT NULL,
    "expires_at" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_user_sessions_by_token_hash" ON "user_sessions" ("token_hash");
-- #[toasty::breakpoint]
CREATE INDEX "index_user_sessions_by_user_id" ON "user_sessions" ("user_id");
-- #[toasty::breakpoint]
CREATE TABLE "shelf_entries" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "shelf_id" INTEGER NOT NULL,
    "book_id" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_shelf_entries_by_shelf_id" ON "shelf_entries" ("shelf_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_shelf_entries_by_book_id" ON "shelf_entries" ("book_id");
-- #[toasty::breakpoint]
CREATE TABLE "read_marks" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "book_id" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_read_marks_by_user_id" ON "read_marks" ("user_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_read_marks_by_book_id" ON "read_marks" ("book_id");
-- #[toasty::breakpoint]
CREATE TABLE "kobo_states" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "book_id" INTEGER NOT NULL,
    "state" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_kobo_states_by_user_id" ON "kobo_states" ("user_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_kobo_states_by_book_id" ON "kobo_states" ("book_id");
