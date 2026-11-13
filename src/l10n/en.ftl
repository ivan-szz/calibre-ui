app_name = Library

## Navigation
nav_books = Books
nav_discover = Discover
nav_shelves = Shelves
nav_read = Read
nav_unread = Unread
nav_upload = Upload
nav_browse = Browse
nav_you = You
nav_tasks = Tasks
nav_account = Account
nav_users = Users
sign_out = Sign out
search_placeholder = Search titles, authors, tags, series…
search_label = Search the library

kind_authors = Authors
kind_series = Series
kind_tags = Tags
kind_publishers = Publishers
kind_languages = Languages
kind_ratings = Ratings
kind_formats = Formats

## Book lists
book_count = { $count ->
    [one] 1 book
   *[other] { $count } books
}
in_library = { $count } in your library
nothing_here = Nothing here yet
no_books = No books here yet
reading_now = Continue reading
percent_read = { $percent }% read
read_badge = Read
read_so_far = read so far
sort_label = Sort
sort_new = Newest
sort_old = Oldest
sort_title = Title
sort_author = Author
sort_pubdate = Published
previous = Previous
next = Next
page_of = Page { $page } of { $pages }
discover_subtitle = A random pick from your library
shuffle = Shuffle
search_title = Search
search_hint = Type in the search box to find books
search_results = Results for “{ $query }”

## Book page
read_book = Read
mark_read = Mark as read
edit = Edit
download = Download
book_of_series = Book { $number } of { $series }
by = by
rating = Rating
publisher = Publisher
published = Published
language = Language
send_to_ereader = Send to e-reader
ereader_missing = Set your e-reader email first, in
format_to_send = Format to send
send_to = Send to { $email }
sent_to = Sent to { $email }
send_failed = Sending failed: { $error }
convert = Convert
convert_from = Convert from
convert_to = Convert to
convert_task = Convert “{ $title }” from { $from } to { $to }
conversion_started = Conversion started
no_shelves = No shelves yet
create_one = Create one

## Reader
previous_page = Previous page
next_page = Next page
page_n = Page { $n }
comic_empty = This comic has no images
continue_reading = Continue
reader_contents = Contents
reader_settings = Text and colours
reader_theme = Page colour
theme_cream = Cream
theme_sepia = Sepia
theme_white = White
theme_dark = Dark
reader_font = Font
font_publisher = The book's own
font_literata = Literata
font_serif = Georgia (serif)
font_sans = Geist (sans serif)
font_atkinson = Atkinson Hyperlegible
font_dyslexic = OpenDyslexic (for dyslexia)
reader_size = Text size
reader_line_height = Line spacing
reader_progress = Progress through the book
reader_page_of = Page { $n } of { $total }
reader_resume = Pick up here
reader_resume_dismiss = Close
reader_keys = ← → or Space to turn pages, T for contents, S for these settings, Esc to close
reader_keys_comic = ← → or Space to turn pages, S for these settings, Esc to close
reader_pdf_note = Your browser shows PDFs with its own viewer, which keeps no place in the book and takes no colours or fonts

## Editing
edit_title = Edit “{ $title }”
edit_subtitle = Changes are written to the library by Calibre
back_to_book = Back to book
field_title = Title
field_series_index = Number in series
field_description = Description
field_cover = Cover
hint_authors = Separate several with “&”
hint_commas = Separate with commas
hint_date = YYYY-MM-DD
hint_languages = ISO codes separated by commas, like ita, eng
hint_cover = A JPEG or PNG image. Leave empty to keep the current cover.
hint_description = Plain text. Leave it unchanged to keep the original formatting.
hint_yes_no = Yes or No
hint_stars = Stars from 0 to 5
hint_number = A number
no_rating = No rating
save_changes = Save changes
fetch_metadata = Fetch metadata
fetch_metadata_hint = Look the book up in Calibre's online sources, then pick what to keep
search_online = Search online
formats_hint = Remove a file from the book
no_files = This book has no files
remove = Remove
delete_book = Delete book
delete_book_hint = Calibre moves it to the library's trash, where it can be restored from the Calibre app for a while
delete_book_open = Delete this book…
delete_book_confirm = Yes, delete “{ $title }”
cover_type = The cover must be a JPEG or PNG image
title_author_required = A book needs a title and an author
changes_saved = Changes saved
no_such_format = The book has no such format
format_removed = Removed { $format }
book_deleted = Deleted “{ $title }”

## Fetching metadata
fetch_title = Metadata for “{ $title }”
fetch_subtitle = Found by Calibre's metadata sources. Pick the fields to use.
back_to_edit = Back to edit
fetch_none = No source found this book. Check its title and authors, or add an ISBN in Calibre.
use_field = Use
field = Field
now = Now
found = Found
use_found = Use the found { $field }
use_found_cover = Use the found cover
has_cover = Has a cover
none = None
download_found_cover = Download the cover the sources have
apply_selected = Apply selected
metadata_applied = Metadata applied

## Shelves
shelves_subtitle = Group books into your own lists
new_shelf_name = New shelf name
create = Create
delete_shelf = Delete shelf
delete_shelf_named = Delete shelf { $name }

## Tasks
tasks_subtitle = Conversions run in the background. Reload to see progress.
no_tasks = No tasks yet

## Upload
upload_subtitle = Calibre reads the metadata and cover from each file
upload_choose = Choose book files
upload_formats = EPUB, PDF, MOBI, AZW3, CBZ, FB2, DOCX and more, up to 512 MB in total
upload_submit = Add to library
upload_drop = or drop them here
upload_sending = Uploading…
upload_not_ebook = not an e-book format Calibre imports
upload_duplicate = already in the library
upload_added_one = Book added
upload_added = Added { $count } books
upload_none = Choose at least one file
upload_partial = Added { $done } of { $tried } files

## Sign-in
login_title = Sign in to your library
login_hint = Use the account your library admin gave you
login_submit = Sign in
login_wrong = Wrong username or password
setup_title = Create the admin account
setup_hint = No accounts exist yet. This one can manage the library and its users.
setup_submit = Create account
username = Username
password = Password
password_rule = Choose a password
account_rule = Choose a username and a password
opds_sign_in = Sign in to read this catalog

## Account
signed_in_as = Signed in as { $name }
ereader_hint = Books are emailed here, such as your Kindle address. Remember to allow the library's sender address.
ereader_email = E-reader email
save = Save
password_change_hint = Changing it signs you out on every device
current_password = Current password
new_password = New password
change_password = Change password
not_an_email = That is not an email address
ereader_saved = E-reader email saved
appearance = Appearance
appearance_hint = The colours of the library, on every device you sign in on
theme = Theme
theme_system = Same as the device
theme_light = Light
theme_dark_ui = Dark
theme_saved = Theme saved
current_password_wrong = The current password is wrong
password_changed = Password changed. Sign in again.

## Users
users_subtitle = Who can use the library, and what they may do
permissions = Permissions
actions = Actions
add_user = Add user
perm_download = Download books
perm_upload = Upload books
perm_edit = Edit and delete books
perm_admin = Admin (everything, and manage users)
perm_download_short = Download
perm_upload_short = Upload
perm_edit_short = Edit
perm_admin_short = Admin
username_taken = That username is taken
user_added = Added { $name }
edit_account = Edit this account
save_permissions = Save permissions
hidden_tags = Hidden tags
hidden_tags_hint = Books with any of these tags are invisible to this user: lists, search, OPDS, and downloads
hidden_tags_rule = Separate with commas. Leave empty to show everything.
hidden_tags_saved = Hidden tags saved
reset_password = Reset password
set_password = Set password
password_set = Password set
delete_account = Delete account
delete_account_hint = Removes the account and its shelves. Books stay in the library.
delete_named = Delete { $name }
deleted_named = Deleted { $name }
cannot_demote_self = You cannot remove your own admin permission
cannot_delete_self = You cannot delete your own account
permissions_saved = Permissions saved

## OPDS and mail
opds_newest = Newest books
opds_newest_hint = The books added last
mail_body = Sent from your library: { $title }

## Single sign-on
or = or
sign_in_with = Sign in with { $name }
oidc_failed = Signing in with the provider did not work. Try again.
oidc_name_taken = The name { $name } belongs to a local account, so it cannot sign in through the provider

## Kobo
kobo_sync = Kobo sync
kobo_hint = Sync your books and reading progress with a Kobo e-reader. Only EPUB and KEPUB books are synced.
kobo_enable = Turn on Kobo sync
kobo_setting = Connect the Kobo to a computer and set this line in .kobo/Kobo/Kobo eReader.conf, under [OneStoreServices]:
kobo_new_link = New link
kobo_disable = Turn off
kobo_enabled = Kobo sync is on. The previous link, if any, no longer works.
kobo_disabled = Kobo sync is off
