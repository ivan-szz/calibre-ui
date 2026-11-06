# calibre-ui

A port of [calibre-web](https://github.com/janeczku/calibre-web) to [Topcoat](https://github.com/tokio-rs/topcoat), with a refreshed interface.

- Browse, search, and sort the library; browse by author, series, tag, publisher, language, rating, and format; discover random books.
- Book pages with metadata, custom columns, downloads, and a reader in the browser (EPUB with epub.js, PDF natively, CBZ comics page by page).
- Accounts with permissions (download, upload, edit, admin) and per-user hidden tags, managed by admins. The first visit creates the admin account.
- Upload books, edit metadata, custom columns and covers, fetch metadata and covers from online sources, remove formats or whole books, convert between formats.
- Send books to an e-reader by email.
- Personal shelves, and read/unread marks.
- An OPDS catalog at `/opds` for e-reader apps, with HTTP Basic sign-in.
- Kobo sync: each user can turn it on in Account and gets a secret `api_endpoint` line for the Kobo's `eReader.conf`. EPUB and KEPUB books sync to the device, with covers, and reading progress syncs back; finishing a book marks it read.
- Optional single sign-on with OpenID Connect, and optional LDAP sign-in. Accounts from either are created on first sign-in with the download permission, and can never take over a local account of the same name.
- English and Italian interface, picked from the browser's languages (`src/l10n/*.ftl`).

Calibre stays the engine behind it. Reads go straight to the library's `metadata.db` and files. Every change goes through Calibre's own tools: `calibredb` (add, edit, delete), `ebook-convert` (conversion), and `fetch-ebook-metadata` (online metadata). Install [Calibre](https://calibre-ebook.com) on the server. The tools are found on `PATH`, in `/Applications/calibre.app` on macOS, or at `CALIBREDB`, `EBOOK_CONVERT`, and `FETCH_EBOOK_METADATA`. `calibredb` cannot write while the Calibre desktop app has the library open, unless its content server is running.

Accounts, sessions, shelves, and reading positions live in the app's own `data.db`.

## Configuration

| Variable | Meaning |
| --- | --- |
| `CALIBRE_LIBRARY` | The library folder, the one containing `metadata.db`. Default `./library`. |
| `DATABASE_URL` | The app's own database. Default `sqlite:data.db`. |
| `CALIBREDB`, `EBOOK_CONVERT`, `FETCH_EBOOK_METADATA` | Paths to the Calibre tools, if not found automatically. |
| `SMTP_URL` | Mail server for "Send to e-reader", like `smtps://user:pass@smtp.example.com:465`. Without it, mail is written to `mail-outbox/`. |
| `MAIL_FROM` | Sender address. Allow it in your e-reader's approved senders. |
| `HOST`, `PORT` | Where to listen. Default `127.0.0.1:3000`. |
| `PUBLIC_URL` | The app's address as users and devices reach it, like `https://books.example.com`. Needed for OIDC; used in Kobo sync links (otherwise taken from the request). |
| `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` | Turn on "Sign in with…" through an OpenID Connect provider. Register `{PUBLIC_URL}/login/oidc/callback` as its redirect URI. |
| `OIDC_NAME` | The provider's name on the sign-in button. Default `SSO`. |
| `KOBO_STORE_PROXY` | `1` sends the Kobo requests this app does not answer on to Kobo's store, as calibre-web's "proxy unknown requests to Kobo Store" option does, so the device's store features keep working. Off by default. |
| `LDAP_URL`, `LDAP_USER_DN` | Turn on LDAP sign-in, like `ldaps://ldap.example.com` and `uid={username},ou=people,dc=example,dc=com`. |

A variable set to an empty value counts as unset.

How each part was tested: library reads, writes, conversion, and online metadata against a real Calibre install; OIDC end to end against a mock provider; Kobo sync and the store proxy by calling the API (the proxy against a mock store), not on a real Kobo device. **LDAP sign-in is untested against a real directory**: only its DN escaping and its refusal of empty passwords have tests.

Sessions use a `Secure` cookie, so serve the app over HTTPS (or on `localhost`).

```sh
CALIBRE_LIBRARY=~/Calibre\ Library topcoat dev
```

## Development

Install the Topcoat CLI:

```sh
cargo install topcoat-cli
```

The app's database is stored in `data.db`. The app creates it and applies the migrations in `toasty/` on every start, so there is nothing to set up. After changing a model, generate a new migration:

```sh
cargo run -- toasty migration generate
```

Start the development server, which rebuilds the application when you edit its source:

```sh
topcoat dev
```

Then open <http://127.0.0.1:3000>.

## Deploying

The `Dockerfile` builds the release binary with its asset bundle, and installs Calibre's command-line tools (`CALIBRE_VERSION`, default 9.18.0) in a Debian image. The container listens on port 3000 and keeps its state in two volumes:

| Path | Contents |
| --- | --- |
| `/data` | `data.db` (accounts, sessions, shelves, reading positions), Calibre's settings, and `mail-outbox/`. |
| `/library` | The Calibre library. The first start on an empty volume creates an empty library. |

The container runs as user 1000. Named volumes get the right owner by themselves; a host folder mounted instead must be writable by that user.

```sh
docker compose up --build
```

### On Coolify

1. Create a resource from this Git repository, with the **Docker Compose** build pack. It reads `docker-compose.yml`.
2. Give the `calibre-ui` service its domain with the container port, like `https://books.example.com:3000`. Coolify serves it over HTTPS, which the session cookie needs.
3. In the Environment tab, set `PUBLIC_URL` to the same address without the port, like `https://books.example.com`, and any of the optional variables above.
4. Deploy, then open the site at once and create the admin account: until one exists, whoever opens the site first gets to create it.

To bring an existing library, copy its folder (with `metadata.db`) into the `/library` volume while the app is stopped.

The **Dockerfile** build pack works too: set the exposed port to 3000, and add persistent storage for `/data` and `/library` by hand.

## Project structure

| Path | Contents |
| --- | --- |
| `src/app.rs`, `src/app/` | The route tree: the router, layouts, pages, and routes. |
| `src/features.rs`, `src/features/` | Everything one feature owns, such as its data and business rules. |
| `src/components.rs`, `src/components/` | Shared UI components that belong to no single feature. |
| `src/db.rs` | The database connection. |
| `toasty/` | The database migrations. Commit them with your code. |

Pages in `app` use features and components. A feature can use shared components but never imports from `app`, and shared components never import from a feature.

## Learn more

- [Getting started](https://github.com/tokio-rs/topcoat/blob/main/docs/getting_started.md)
- [Guides](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat)
