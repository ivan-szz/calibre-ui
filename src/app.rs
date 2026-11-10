mod account;
mod admin;
mod books;
mod browse;
mod discover;
mod kobo;
mod login;
mod opds;
mod reading;
mod search;
mod shelves;
mod tasks;
mod ui;
mod unread;
mod upload;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    font::{Font, fontsource::fontsource_font},
    icon::{IconData, icon, iconify::iconify_icon},
    l10n::{Locale, Translation, accept_language, locale, negotiate},
    mail::RouterBuilderMailExt,
    router::{
        Body, BodyLimit, Next, RouterBuilder, RouterBuilderDiscoverExt, Slot, StatusCode, header,
        error::{RouterErrorExt, redirect},
        href, layer, layout, module_router, page, request,
        response::{IntoResponse, Response},
    },
    runtime::RouterBuilderRuntimeExt,
    session::RouterBuilderSessionExt,
    tailwind,
    view::{View, attributes, class, component, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button},
        input::input,
    },
    features::{
        auth::{self, Perm, User},
        library::{Filter, Kind},
        mailer,
    },
    t,
};
use browse::category::Category;

/// The Geist font family, loaded from Fontsource.
const GEIST: Font = fontsource_font!(GEIST, weight: [400, 500, 600, 700], style: Normal);

/// The interface languages, from `src/l10n`. The first is the fallback.
const LOCALES: [Locale; 2] = [locale!("en"), locale!("it")];

/// Builds the application's router.
pub fn router() -> RouterBuilder {
    // The pages and layouts of this module and its submodules, with paths derived
    // from the module tree.
    module_router!()
        // The runtime's procedures and shards, which have no place in the module tree.
        .discover()
        .assets(AssetBundle::load().unwrap())
        .cookies()
        .sessions(auth::session_config())
        .mail(mailer::config())
        // Book files can be far larger than the default 2 MiB.
        .layer(BodyLimit::max(512 * 1024 * 1024).at("/upload"))
        .layer(BodyLimit::max(32 * 1024 * 1024).at("/books"))
        // Lets the browser runtime rerun pages; register it after other layers.
        .runtime()
}

/// The signed-in user if they have `perm`, or a 403.
pub async fn require(cx: &Cx, perm: Perm) -> Result<&User> {
    Ok(auth::current_user(cx).await.filter(|u| u.can(perm)).ok_or_forbidden()?)
}

/// The tags whose books the signed-in user cannot see.
pub async fn hidden_tags(cx: &Cx) -> Vec<String> {
    auth::current_user(cx).await.map(User::hidden).unwrap_or_default()
}

/// The signed-in user, or a 401.
pub async fn signed_in(cx: &Cx) -> Result<&User> {
    Ok(auth::current_user(cx).await.ok_or_unauthorized()?)
}

/// Picks the interface language from the browser, then sends anonymous visitors to the
/// login page. OPDS clients get a Basic auth challenge instead, since they cannot
/// follow a login form.
#[layer]
async fn require_login(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let chosen = negotiate(accept_language(cx), &LOCALES).cloned().unwrap_or(locale!("en"));
    let cx = &cx.with(chosen);
    let path = request::uri(cx).path();
    // Kobo devices carry no session; their URL token is checked by each Kobo route.
    let public = path == "/login"
        || path.starts_with("/login/")
        || path.starts_with("/kobo/")
        || path.starts_with("/_topcoat/assets/");
    if public || auth::current_user(cx).await.is_some() {
        return next.run(cx, body).await;
    }
    if path.starts_with("/opds") {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Library\", charset=\"UTF-8\"")],
            t!(opds_sign_in).resolve(cx).into_owned(),
        )
            .into_response(cx);
    }
    Err(redirect(href!(login::page).resolve(cx)).into())
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    // Keeps the search text in the box; an invalid query string is the page's to report.
    let search = topcoat::router::query_params::<ui::ListQuery>(cx).ok().and_then(|q| q.q.clone());
    let user = auth::current_user(cx).await.cloned();
    // The reader fills the window with the book alone.
    let reading = books::book_id::read::is_reader(request::uri(cx).path());
    Ok(view! {
        <!DOCTYPE html>
        <html lang=(locale(cx).id()) dir=(locale(cx).id().direction())>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(t!(app_name))</title>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <link rel="search" type="application/opensearchdescription+xml" href="/opds/osd">
                topcoat::font::link(font: GEIST)
                topcoat::runtime::script()
                topcoat::dev::script()
            </head>
            <body class="min-h-screen md:flex">
                match user {
                    Some(_) if reading => (slot),
                    None => <main class="flex min-h-screen flex-1 flex-col items-center justify-center px-4 py-12">
                        <div class="w-full max-w-sm">ui::notice()</div>
                        (slot)
                    </main>,
                    Some(user) => {
                        <aside class="border-b border-sidebar-border bg-sidebar text-sidebar-foreground md:sticky md:top-0 md:flex md:h-screen md:w-60 md:shrink-0 md:flex-col md:overflow-y-auto md:border-r md:border-b-0">
                            <a href=(href!(home)) class="flex items-center gap-2 px-5 pt-5 pb-3 text-lg font-semibold tracking-tight">
                                <span class="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
                                    icon(data: iconify_icon!("lucide:library-big"))
                                </span>
                                (t!(app_name))
                            </a>
                            <nav class="flex gap-1 overflow-x-auto px-3 pb-3 md:flex-1 md:flex-col md:pb-4">
                                nav_link(target: href!(home).resolve(cx), data: iconify_icon!("lucide:book-open"), label: t!(nav_books))
                                nav_link(target: href!(discover::page).resolve(cx), data: iconify_icon!("lucide:sparkles"), label: t!(nav_discover))
                                nav_link(target: href!(shelves::page).resolve(cx), data: iconify_icon!("lucide:bookmark"), label: t!(nav_shelves))
                                nav_link(target: href!(reading::page).resolve(cx), data: iconify_icon!("lucide:book-check"), label: t!(nav_read))
                                nav_link(target: href!(unread::page).resolve(cx), data: iconify_icon!("lucide:book-dashed"), label: t!(nav_unread))
                                if user.can(Perm::Upload) {
                                    nav_link(target: href!(upload::page).resolve(cx), data: iconify_icon!("lucide:upload"), label: t!(nav_upload))
                                }
                                <p class="hidden px-3 pt-5 pb-1 text-xs font-medium tracking-wide text-muted-foreground uppercase md:block">(t!(nav_browse))</p>
                                #[key(kind.to_string())]
                                for kind in Kind::ALL {
                                    nav_link(
                                        target: href!(browse::category::page, Category(kind)).resolve(cx),
                                        data: kind_icon(kind),
                                        label: kind_label(kind),
                                    )
                                }
                                <p class="hidden px-3 pt-5 pb-1 text-xs font-medium tracking-wide text-muted-foreground uppercase md:block">(t!(nav_you))</p>
                                nav_link(target: href!(tasks::page).resolve(cx), data: iconify_icon!("lucide:list-checks"), label: t!(nav_tasks))
                                nav_link(target: href!(account::page).resolve(cx), data: iconify_icon!("lucide:user-round"), label: t!(nav_account))
                                if user.admin {
                                    nav_link(target: href!(admin::users::page).resolve(cx), data: iconify_icon!("lucide:shield"), label: t!(nav_users))
                                }
                            </nav>
                            <form method="post" action=(href!(login::logout::logout)) class="hidden items-center justify-between gap-2 border-t border-sidebar-border px-5 py-3 md:flex">
                                <span class="truncate text-sm font-medium">(&user.name)</span>
                                button(
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    attrs: attributes! { type="submit" title=(t!(sign_out)) aria-label=(t!(sign_out)) },
                                    icon(data: iconify_icon!("lucide:log-out"))
                                )
                            </form>
                        </aside>
                        <div class="min-w-0 flex-1">
                            <header class="sticky top-0 z-10 border-b border-border bg-background/80 px-4 py-3 backdrop-blur md:px-8">
                                <form action=(href!(search::page)) class="relative max-w-xl" role="search">
                                    <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground">
                                        icon(data: iconify_icon!("lucide:search"))
                                    </span>
                                    input(attrs: attributes! {
                                        type="search"
                                        name="q"
                                        placeholder=(t!(search_placeholder))
                                        aria-label=(t!(search_label))
                                        value=(search)
                                        class="pl-9"
                                    })
                                </form>
                            </header>
                            <main class="px-4 py-8 md:px-8">ui::notice() (slot)</main>
                        </div>
                    },
                }
            </body>
        </html>
    })
}

/// A sidebar link, highlighted when it points at the current page or one below it.
#[component]
async fn nav_link(cx: &Cx, target: String, data: IconData, label: Translation<'static>) -> Result<impl View> {
    let path = request::uri(cx).path();
    let current = path == target || (target != "/" && path.starts_with(&format!("{target}/")));
    Ok(view! {
        <a
            href=(&target)
            aria-current=(current.then_some("page"))
            class=(class!(
                "flex shrink-0 items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                "hover:bg-sidebar-accent",
                "bg-sidebar-accent text-sidebar-accent-foreground" if current,
                "text-sidebar-foreground/80" if !current,
            ))
        >
            icon(data: data)
            (label)
        </a>
    })
}

/// The sidebar icon for a category.
pub fn kind_icon(kind: Kind) -> IconData {
    match kind {
        Kind::Authors => iconify_icon!("lucide:users"),
        Kind::Series => iconify_icon!("lucide:layers"),
        Kind::Tags => iconify_icon!("lucide:tags"),
        Kind::Publishers => iconify_icon!("lucide:building-2"),
        Kind::Languages => iconify_icon!("lucide:languages"),
        Kind::Ratings => iconify_icon!("lucide:star"),
        Kind::Formats => iconify_icon!("lucide:file-type"),
    }
}

/// The name of a category in the interface language.
pub fn kind_label(kind: Kind) -> Translation<'static> {
    match kind {
        Kind::Authors => t!(kind_authors),
        Kind::Series => t!(kind_series),
        Kind::Tags => t!(kind_tags),
        Kind::Publishers => t!(kind_publishers),
        Kind::Languages => t!(kind_languages),
        Kind::Ratings => t!(kind_ratings),
        Kind::Formats => t!(kind_formats),
    }
}

/// Every book in the library, newest first.
#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        ui::book_list(title: t!(nav_books).resolve(cx).into_owned(), filter: Filter::All, default_sort: "new")
    })
}
