use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{View, view},
};

use super::{signed_in, ui::heading};
use crate::{features::calibre::{TaskStatus, tasks}, t};

/// Background jobs such as conversions. Admins see everyone's.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = signed_in(cx).await?;
    let (id, admin) = (user.id, user.admin);
    let tasks: Vec<_> = tasks().into_iter().filter(|t| admin || t.user_id == id).collect();
    Ok(view! {
        heading(title: t!(nav_tasks).resolve(cx), subtitle: t!(tasks_subtitle).resolve(cx))
        if tasks.is_empty() {
            <p class="mt-8 text-muted-foreground">(t!(no_tasks))</p>
        } else {
            <ul class="mt-8 max-w-2xl divide-y divide-border rounded-xl border border-border bg-card shadow-sm">
                for task in tasks {
                    <li class="flex items-start gap-3 px-4 py-3 text-sm">
                        match &task.status {
                            TaskStatus::Running => <span class="mt-0.5 animate-spin text-muted-foreground">icon(data: iconify_icon!("lucide:loader-circle"))</span>,
                            TaskStatus::Done => <span class="mt-0.5 text-emerald-600">icon(data: iconify_icon!("lucide:circle-check"))</span>,
                            TaskStatus::Failed(_) => <span class="mt-0.5 text-destructive">icon(data: iconify_icon!("lucide:circle-x"))</span>,
                        }
                        <div class="min-w-0">
                            <p class="font-medium">(&task.label)</p>
                            match task.status {
                                TaskStatus::Failed(error) => <p class="mt-1 text-xs break-words text-muted-foreground">(error)</p>,
                                _ => "",
                            }
                        </div>
                    </li>
                }
            </ul>
        }
    })
}
