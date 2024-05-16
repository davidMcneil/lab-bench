use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons::{
    FaBan, FaCaretDown, FaCaretRight, FaCircleCheck, FaCircleExclamation, FaCircleQuestion,
    FaCodeBranch, FaCodeMerge, FaComment, FaListCheck, FaSpinner,
};
use dioxus_free_icons::Icon;
use timeago::Formatter;
use tracing::{info, Level};
use strum::IntoEnumIterator;

use crate::api::{
    fetch_merge_requests, fetch_merge_requests_with_full_data, MergeRequest, MergeRequestsDomain,
    MergeRequestsQuery, OrderBy, Scope, Sort,
};

mod api;

fn main() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App)
}

#[component]
fn App() -> Element {
    info!("lab-bench 9");

    let initial_gitlab_url = "https://gitlab.com/api/v4";
    let initial_private_token = "";

    // Inputs
    let mut gitlab_url = use_signal(|| initial_gitlab_url.to_string());
    let mut private_token = use_signal(|| initial_private_token.to_string());
    let mut query_expanded = use_signal(|| true);
    // TODO: on input update the `query` and`domains` signals dynamically
    let mut query = use_signal(|| MergeRequestsQuery {
        created_after: None,
        created_before: None,
        order_by: OrderBy::default(),
        scope: Scope::All,
        sort: Sort::default(),
        state: None,
        updated_after: None,
        updated_before: None,
        wip: None,
    });
    let mut author_domains = use_signal(|| {vec![]});
    let mut project_domains = use_signal(|| {vec![]});

    // Outputs
    let mut merge_requests_result = use_signal(|| Ok::<_, String>(Vec::new()));

    rsx! {
        div { class: "max-w-screen-lg mx-auto mt-1",
            div { class: "flex flex-row justify-between",
                div { class: "flex flex-row items-center",
                    h1 { class: "font-ariel text-2xl mr-1", "Lab Bench" }
                    span {
                        class: "cursor-pointer",
                        onclick: move |_| *query_expanded.write() = !query_expanded(),
                        if query_expanded() {
                            Icon { width: 18, height: 18, icon: FaCaretDown }
                        } else {
                            Icon { width: 18, height: 18, icon: FaCaretRight }
                        }
                    }
                }
                div { class: "flex flex-row items-center",
                    if let Ok(r) = merge_requests_result() {
                        span { class: "font-ariel text-lg mr-1", "{r.len()}" }
                    }
                    button {
                        class: "px-4 py-1 border rounded-sm border-gray-300 bg-gray-100",
                        prevent_default: "onclick",
                        onclick: move |_event| {
                            spawn(async move {
                                let mut domains = author_domains();
                                domains.append(&mut project_domains().clone());
                                *merge_requests_result
                                    .write() = fetch_merge_requests(
                                        &gitlab_url(),
                                        &private_token(),
                                        &query(),
                                        &domains,
                                    )
                                    .await
                                    .map_err(|e| e.to_string());
                                if let Ok(merge_requests) = merge_requests_result() {
                                    *merge_requests_result
                                        .write() = fetch_merge_requests_with_full_data(
                                            &gitlab_url(),
                                            &private_token(),
                                            &merge_requests,
                                        )
                                        .await
                                        .map_err(|e| e.to_string());
                                }
                            });
                        },
                        "Query"
                    }
                }
            }
            // Query builder
            // TODO: format this nicely
            div { class: "flex flex-col",
                form { class: if query_expanded() { "" } else { "hidden" },
                    div { class: "flex flex-row",
                        label { class: "block", "GitLab Url" }
                        input {
                            r#type: "text",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            value: initial_gitlab_url,
                            oninput: move |event| {
                                *gitlab_url.write() = event.value();
                            }
                        }
                        label { class: "block", "Private Token" }
                        input {
                            r#type: "password",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            value: initial_private_token,
                            oninput: move |event| {
                                *private_token.write() = event.value();
                            }
                        }
                    }
                    div { class: "flex flex-row",

                        label { class: "block", "Start" }
                        input {
                            r#type: "text",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            oninput: move |_event| { todo!() }
                        }
                        label { class: "block", "End" }
                        input {
                            r#type: "text",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            oninput: move |_event| { todo!() }
                        }
                    }
                    div { class: "flex flex-row",
                        label { class: "block", "Repos" }
                        input {
                            r#type: "text",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            oninput: move |event| {
                                *project_domains.write() = event.value().split_whitespace().map(|x| MergeRequestsDomain::ProjectPath(x.to_string())).collect();
                            }
                        }
                        label { class: "block", "Authors" }
                        input {
                            r#type: "text",
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            oninput: move |event| {
                                *author_domains.write() = event.value().split_whitespace().map(|x| MergeRequestsDomain::AuthorUsername(x.to_string())).collect();
                            }
                        }
                    }
                    div { class: "flex flex-row",
                        label { class: "block", "Sort" }
                        select {
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            onchange: move |event| {
                                (*query.write()).sort = serde_json::from_str(&event.value()).unwrap();
                            },
                            for x in api::Sort::iter() {
                                option {
                                    value: serde_json::to_string(&x).unwrap(),
                                    {remove_first_and_last_chars(&serde_json::to_string(&x).unwrap())}
                                }
                            }
                        }
                        label { class: "block", "Order By" }
                        select {
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            onchange: move |event| {
                                (*query.write()).order_by = serde_json::from_str(&event.value()).unwrap();
                            },
                            for x in api::OrderBy::iter() {
                                option {
                                    value: serde_json::to_string(&x).unwrap(),
                                    {remove_first_and_last_chars(&serde_json::to_string(&x).unwrap())}
                                }
                            }
                        }
                        label { class: "block", "Scope" }
                        select {
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            onchange: move |event| {
                                (*query.write()).scope = serde_json::from_str(&event.value()).unwrap();
                            },
                            for x in api::Scope::iter() {
                                option {
                                    value: serde_json::to_string(&x).unwrap(),
                                    {remove_first_and_last_chars(&serde_json::to_string(&x).unwrap())}
                                }
                            }
                        }
                        label { class: "block", "State" }
                        select {
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            onchange: move |event| {
                                (*query.write()).state = serde_json::from_str(&event.value()).ok();
                            },
                            option {
                                value: "",
                                ""
                            },
                            for x in api::State::iter() {
                                option {
                                    value: serde_json::to_string(&x).unwrap(),
                                    {remove_first_and_last_chars(&serde_json::to_string(&x).unwrap())}
                                }
                            }
                        }
                        label { class: "block", "Wip" }
                        select {
                            class: "block p-1 border rounded-sm border-gray-300 bg-gray-100 text-xs text-ariel",
                            onchange: move |event| {
                                (*query.write()).wip = serde_json::from_str(&event.value()).ok();
                            },
                            option {
                                value: "",
                                ""
                            },
                            for x in api::Wip::iter() {
                                option {
                                    value: serde_json::to_string(&x).unwrap(),
                                    {remove_first_and_last_chars(&serde_json::to_string(&x).unwrap())}
                                }
                            }
                        }
                    }
                }
            }
            // MR list
            match merge_requests_result.read().clone(){
                Ok(merge_request_list) =>  rsx!(MergeRequestList { merge_request_list }),
                Err(e) => rsx!(span {"{e}"}),
            }
        }
    }
}

fn remove_first_and_last_chars(s: &str) -> &str {
    &s[1..s.len() - 1]
}

#[component]
fn MergeRequestList(merge_request_list: Vec<MergeRequest>) -> Element {
    rsx!(
        ul { class: "list-none",
            for merge_request in merge_request_list {
                li { key: "{merge_request.references.full}", class: "flex flex-col py-1 border-b",
                    MergeRequest { merge_request }
                }
            }
        }
    )
}

#[component]
fn MergeRequest(merge_request: MergeRequest) -> Element {
    use crate::api::{
        MergeStatus::{self, *},
        PipelineStatus::{self, *},
        State::{self, *},
    };

    let MergeRequest {
        author,
        created_at,
        detailed_merge_status,
        head_pipeline,
        merge_when_pipeline_succeeds,
        references,
        reviewers,
        source_branch,
        state,
        title,
        updated_at,
        user_notes_count,
        web_url,
        ..
    } = merge_request;

    let head_pipeline: api::Pipeline = head_pipeline.unwrap_or_default();
    let pipeline_time_in_min = head_pipeline.duration.num_minutes();
    let pipeline_queued_time_in_min = head_pipeline.queued_duration.num_minutes();

    rsx!(
        div { class: "flex flex-row justify-between",
            // Left column
            div { class: "flex flex-col",
                div { class: "flex flex-row items-center",
                    a {
                        class: "font-ariel text-sm mr-1",
                        href: web_url.as_ref(),
                        "{title}"
                    }
                    span {
                        class: "cursor-pointer",
                        title: source_branch.as_ref(),
                        onclick: move |_event| { set_clipboard(&source_branch) },
                        Icon { width: 16, height: 16, title: source_branch.as_str(), icon: FaCodeBranch }
                    }
                }
                div { class: "flex flex-row items-center",
                    span { class: "font-ariel text-xs mr-1", "{references.full}" }
                    div { class: "font-ariel text-xs",
                        span { class: "mr-1", title: created_at.to_string(),
                            "created {time_ago(created_at)} by"
                        }
                        a { href: author.web_url, "{author.username}" }
                    }
                }
            }
            // Right column
            div { class: "flex flex-col",
                div { class: "flex flex-row items-center justify-end items-center",
                    // Merge status
                    a {
                        class: "mr-1",
                        href: web_url,
                        title: "{state}:{detailed_merge_status}",
                        match (merge_when_pipeline_succeeds, state, detailed_merge_status) {
                            (_, _, MergeStatus::Unknown) | (_, State::Unknown, _) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCircleQuestion,
                                fill: "#dd2b0e",
                            }),
                            (_, Closed | Locked, _) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaBan,
                                fill: "#dd2b0e",
                            }),
                            (_, Opened, BlockedStatus | DraftStatus | JiraAssociationMissing | NeedRebase | Conflict
                            | DiscussionsNotResolved | NotApproved | RequestedChanges | Checking | Unchecked | CiMustPass
                            | CiStillRunning | ExternalStatusChecks | NotOpen) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaListCheck,
                                fill: "#1f75cb",
                            }),
                            (true, Opened, Mergeable) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaSpinner,
                                fill: "#108548",
                            }),
                            (false, Opened, Mergeable) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCircleCheck,
                                fill: "#108548",
                            }),
                            (_, Merged, _) => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCodeMerge,
                                fill: "#108548",
                            }),
                        }
                    }
                    // Comments
                    div {
                        class: "flex flex-row items-center font-ariel text-sm",
                        title: "comments",
                        span { class: "mr-1", "{user_notes_count}" }
                        Icon { width: 12, height: 12, fill: "#626168", icon: FaComment }
                    }
                    span { class: "mx-2", "|" }
                    // Pipeline status
                    a {
                        class: "mr-1",
                        title: "pipeline:{head_pipeline.status}",
                        href: head_pipeline.web_url,
                        match head_pipeline.status {
                            PipelineStatus::Unknown => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCircleQuestion,
                                fill: "#dd2b0e",
                            }),
                            Failed => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCircleExclamation,
                                fill: "#dd2b0e",
                            }),
                            Canceled => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaBan,
                                fill: "#dd2b0e",
                            }),
                            Created | WaitingForResource | Preparing | Pending
                            | Running | Skipped | Manual | Scheduled => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaSpinner,
                                fill: "#1f75cb",
                            }),
                            Success => rsx!(Icon {
                                width: 16,
                                height: 16,
                                icon: FaCircleCheck,
                                fill: "#108548",
                            }),
                        }
                    }
                    // Pipeline time
                    span {
                        class: "font-ariel text-sm mr-1",
                        title: "duration: {pipeline_time_in_min} queued: {pipeline_queued_time_in_min}",
                        "{pipeline_time_in_min}m"
                    }
                }
                div { class: "flex flex-row justify-end",
                    span {
                        class: "font-ariel text-xs",
                        title: updated_at.to_string(),
                        "updated {time_ago(updated_at)}"
                    }
                }
            }
        }
        div { class: "flex flex-row items-center",
            span { class: "font-ariel text-xs mr-1", "reviewers:" }
            if reviewers.is_empty() {
                span { class: "font-ariel text-xs", "none" }
            }
            for reviewer in reviewers {
                a { class: "font-ariel text-xs mr-1", href: reviewer.web_url, "{reviewer.username}" }
            }
        }
    )
}

fn time_ago(time: DateTime<Utc>) -> String {
    static FORMATTER: OnceLock<Formatter> = OnceLock::new();
    let formatter = FORMATTER.get_or_init(|| Formatter::new());
    formatter.convert((Utc::now() - time).to_std().unwrap())
}

fn set_clipboard(v: &str) {
    let navigator = web_sys::window().expect("window to exist").navigator();
    let _p = navigator
        .clipboard()
        .expect("clipboard to exist")
        .write_text(v);
}
