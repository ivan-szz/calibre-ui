app_name = Biblioteca

## Navigazione
nav_books = Libri
nav_discover = Scopri
nav_shelves = Scaffali
nav_read = Letti
nav_unread = Da leggere
nav_upload = Carica
nav_browse = Sfoglia
nav_you = Tu
nav_tasks = Attività
nav_account = Account
nav_users = Utenti
sign_out = Esci
search_placeholder = Cerca titoli, autori, tag, serie…
search_label = Cerca nella biblioteca

kind_authors = Autori
kind_series = Serie
kind_tags = Tag
kind_publishers = Editori
kind_languages = Lingue
kind_ratings = Valutazioni
kind_formats = Formati

## Elenchi di libri
book_count = { $count ->
    [one] 1 libro
   *[other] { $count } libri
}
in_library = { $count } nella tua biblioteca
nothing_here = Ancora niente qui.
no_books = Ancora nessun libro qui.
sort_label = Ordina
sort_new = Più recenti
sort_old = Meno recenti
sort_title = Titolo
sort_author = Autore
sort_pubdate = Pubblicazione
previous = Precedente
next = Successiva
page_of = Pagina { $page } di { $pages }
discover_subtitle = Una scelta a caso dalla tua biblioteca.
shuffle = Rimescola
search_title = Cerca
search_hint = Scrivi nella casella di ricerca per trovare dei libri.
search_results = Risultati per “{ $query }”

## Pagina del libro
read_book = Leggi
mark_read = Segna come letto
edit = Modifica
download = Scarica
book_of_series = Libro { $number } di { $series }
by = di
rating = Valutazione
publisher = Editore
published = Pubblicato
language = Lingua
send_to_ereader = Invia all'e-reader
ereader_missing = Prima imposta l'e-mail del tuo e-reader, in
format_to_send = Formato da inviare
send_to = Invia a { $email }
sent_to = Inviato a { $email }.
send_failed = Invio non riuscito: { $error }
convert = Converti
convert_from = Converti da
convert_to = Converti in
convert_task = Conversione di “{ $title }” da { $from } a { $to }
conversion_started = Conversione avviata.
no_shelves = Ancora nessuno scaffale.
create_one = Creane uno

## Lettore
previous_page = Pagina precedente
next_page = Pagina successiva
page_n = Pagina { $n }
comic_empty = Questo fumetto non contiene immagini.

## Modifica
edit_title = Modifica “{ $title }”
edit_subtitle = Le modifiche vengono scritte nella biblioteca da Calibre.
back_to_book = Torna al libro
field_title = Titolo
field_series_index = Numero nella serie
field_description = Descrizione
field_cover = Copertina
hint_authors = Separa più autori con “&”.
hint_commas = Separa con virgole.
hint_date = AAAA-MM-GG
hint_languages = Codici ISO separati da virgole, come ita, eng.
hint_cover = Un'immagine JPEG o PNG. Lascia vuoto per tenere la copertina attuale.
hint_description = Testo semplice. Lascialo invariato per mantenere la formattazione originale.
hint_yes_no = Sì o No.
hint_stars = Stelle da 0 a 5.
hint_number = Un numero.
no_rating = Nessuna valutazione
save_changes = Salva le modifiche
fetch_metadata = Recupera i metadati
fetch_metadata_hint = Cerca il libro nelle fonti online di Calibre, poi scegli cosa tenere.
search_online = Cerca online
formats_hint = Rimuovi un file dal libro.
no_files = Questo libro non ha file.
remove = Rimuovi
delete_book = Elimina il libro
delete_book_hint = Calibre lo sposta nel cestino della biblioteca, da cui per un po' si può ripristinare con l'app Calibre.
delete_book_open = Elimina questo libro…
delete_book_confirm = Sì, elimina “{ $title }”
cover_type = La copertina deve essere un'immagine JPEG o PNG.
title_author_required = Un libro ha bisogno di un titolo e di un autore.
changes_saved = Modifiche salvate.
no_such_format = Il libro non ha questo formato.
format_removed = { $format } rimosso.
book_deleted = “{ $title }” eliminato.

## Recupero dei metadati
fetch_title = Metadati per “{ $title }”
fetch_subtitle = Trovati dalle fonti di metadati di Calibre. Scegli i campi da usare.
back_to_edit = Torna alla modifica
fetch_none = Nessuna fonte ha trovato questo libro. Controlla titolo e autori, o aggiungi un ISBN in Calibre.
use_field = Usa
field = Campo
now = Attuale
found = Trovato
use_found = Usa { $field } trovato
use_found_cover = Usa la copertina trovata
has_cover = Ha una copertina
none = Nessuna
download_found_cover = Scarica la copertina delle fonti
apply_selected = Applica i campi scelti
metadata_applied = Metadati applicati.

## Scaffali
shelves_subtitle = Raggruppa i libri in elenchi tuoi.
new_shelf_name = Nome del nuovo scaffale
create = Crea
delete_shelf = Elimina lo scaffale
delete_shelf_named = Elimina lo scaffale { $name }

## Attività
tasks_subtitle = Le conversioni vengono eseguite in background. Ricarica la pagina per vedere l'avanzamento.
no_tasks = Ancora nessuna attività.

## Caricamento
upload_subtitle = Calibre legge metadati e copertina da ogni file.
upload_choose = Scegli i file dei libri
upload_formats = EPUB, PDF, MOBI, AZW3, CBZ, FB2, DOCX e altri, fino a 512 MB in totale.
upload_submit = Aggiungi alla biblioteca
upload_not_ebook = non è un formato di e-book che Calibre importa
upload_added_one = Libro aggiunto.
upload_added = { $count } libri aggiunti.
upload_none = Scegli almeno un file.
upload_partial = Aggiunti { $done } file su { $tried }.

## Accesso
login_title = Accedi alla tua biblioteca
login_hint = Usa l'account che ti ha dato l'amministratore della biblioteca.
login_submit = Accedi
login_wrong = Nome utente o password errati.
setup_title = Crea l'account amministratore
setup_hint = Non esiste ancora nessun account. Questo potrà gestire la biblioteca e i suoi utenti.
setup_submit = Crea l'account
username = Nome utente
password = Password
password_rule = Almeno 8 caratteri.
account_rule = Scegli un nome utente e una password di almeno 8 caratteri.
opds_sign_in = Accedi per leggere questo catalogo.

## Account
signed_in_as = Hai effettuato l'accesso come { $name }
ereader_hint = I libri vengono inviati per e-mail a questo indirizzo, ad esempio quello del tuo Kindle. Ricorda di autorizzare l'indirizzo mittente della biblioteca.
ereader_email = E-mail dell'e-reader
save = Salva
password_change_hint = Cambiarla ti disconnette da tutti i dispositivi.
current_password = Password attuale
new_password = Nuova password
change_password = Cambia password
not_an_email = Questo non è un indirizzo e-mail.
ereader_saved = E-mail dell'e-reader salvata.
current_password_wrong = La password attuale è errata.
password_changed = Password cambiata. Accedi di nuovo.

## Utenti
users_subtitle = Chi può usare la biblioteca, e cosa può fare.
permissions = Permessi
actions = Azioni
add_user = Aggiungi utente
perm_download = Scaricare i libri
perm_upload = Caricare i libri
perm_edit = Modificare ed eliminare i libri
perm_admin = Amministratore (tutto, e gestione utenti)
perm_download_short = Download
perm_upload_short = Caricamento
perm_edit_short = Modifica
perm_admin_short = Amministratore
username_taken = Questo nome utente è già in uso.
user_added = { $name } aggiunto.
edit_account = Modifica questo account.
save_permissions = Salva i permessi
hidden_tags = Tag nascosti
hidden_tags_hint = I libri con uno di questi tag sono invisibili a questo utente: negli elenchi, nella ricerca, in OPDS e nei download.
hidden_tags_rule = Separa con virgole. Lascia vuoto per mostrare tutto.
hidden_tags_saved = Tag nascosti salvati.
reset_password = Reimposta la password
set_password = Imposta la password
password_set = Password impostata.
delete_account = Elimina l'account
delete_account_hint = Rimuove l'account e i suoi scaffali. I libri restano nella biblioteca.
delete_named = Elimina { $name }
deleted_named = { $name } eliminato.
cannot_demote_self = Non puoi toglierti il permesso di amministratore.
cannot_delete_self = Non puoi eliminare il tuo account.
permissions_saved = Permessi salvati.

## OPDS ed e-mail
opds_newest = Libri più recenti
opds_newest_hint = Gli ultimi libri aggiunti
mail_body = Inviato dalla tua biblioteca: { $title }

## Accesso unico
or = oppure
sign_in_with = Accedi con { $name }
oidc_failed = L'accesso tramite il provider non è riuscito. Riprova.
oidc_name_taken = Il nome { $name } appartiene a un account locale, quindi non può accedere tramite il provider.

## Kobo
kobo_sync = Sincronizzazione Kobo
kobo_hint = Sincronizza libri e avanzamento della lettura con un e-reader Kobo. Vengono sincronizzati solo i libri EPUB e KEPUB.
kobo_enable = Attiva la sincronizzazione Kobo
kobo_setting = Collega il Kobo a un computer e imposta questa riga in .kobo/Kobo/Kobo eReader.conf, nella sezione [OneStoreServices]:
kobo_new_link = Nuovo link
kobo_disable = Disattiva
kobo_enabled = Sincronizzazione Kobo attiva. L'eventuale link precedente non funziona più.
kobo_disabled = Sincronizzazione Kobo disattivata.
