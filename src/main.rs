#![allow(non_snake_case)]

use sycamore::prelude::*;
use sycamore_router::{navigate, HistoryIntegration, Route, Router};
use wasm_bindgen::prelude::*;
use web_sys::{console, window};
use pulldown_cmark as md;
use uuid::Uuid;
use tracing::debug;

use common::*;

macro_rules! wasm_log {
    ( $($t:tt)* ) => {
        console::log_1( &format!( $($t)*) .into() )
    }
}

#[derive(Debug, Route)]
enum AppRoutes {
    #[to("/")]
    Home,
    #[to("/about")]
    About,
    #[to("/chats/<id..>")]
    ChatApp { id: Vec<String> },
    #[to("/codeassist")]
    CodeAssist,
    #[to("/game")]
    TextGame,
    #[not_found]
    NotFound,
}

#[component]
fn NotFound<G: Html>(ctx: Scope) -> View<G> {
    view! { ctx,
        h1 { "404" }
    }
}

#[component]
fn About<G: Html>(ctx: Scope) -> View<G> {
    view! { ctx,
        h1 { "copyright @ sonald (yinshuiboy@gmail.com)" }
    }
}

#[derive(Debug, Clone)]
struct Conversation<'a> {
    id: &'a Signal<Option<ConversationId>>,
    title: &'a Signal<String>,
    chats: &'a Signal<Vec<Message>>,
}

impl<'a> Conversation<'a> {
    pub fn new(ctx: Scope<'a>) -> Self {
        Conversation {
            id: create_signal(ctx, None),
            title: create_signal(ctx, "".to_string()),
            chats: create_signal(ctx, Vec::default()),
        }
    }
}

fn markdown_to_html<S: AsRef<str>>(md: S) -> String {
    let p = md::Parser::new(md.as_ref());
    let mut html_str = String::new();
    md::html::push_html(&mut html_str, p);

    html_str
}

#[derive(Prop)]
struct BubbleProps {
    actor: String,
    at_start: bool,
    content: String,
}

#[component]
fn Bubble<G: Html>(ctx: Scope, props: BubbleProps) -> View<G> {
    let html_content = markdown_to_html(&props.content);
    if props.at_start {
        view! {
            ctx,
            div(class="chat chat-start") {
                div(class="chat-image avatar") {
                    label(class="btn btn-circle rounded-full bg-slate-200") { (props.actor) }
                }
                div(class=("chat-bubble chat-bubble-secondary"),
                dangerously_set_inner_html=&html_content) 
            }
        }

    } else {
        view! {
            ctx,
            div(class="chat chat-end") {
                div(class="chat-image avatar") {
                    label(class="btn btn-circle rounded-full bg-slate-200") { (props.actor) }
                }
                div(class="chat-bubble chat-bubble-success",
                dangerously_set_inner_html=&html_content) 
            }
        }
    }
}

async fn start_conversation(system_hint: String) {
    if let Err(e) = openai_start_conversation(Some(system_hint))
        .await
        .map_err(|e| format!("{:?}", e))
        .and_then(|id| {
            wasm_log!("created: {:?}", id);
            serde_wasm_bindgen::from_value::<ConversationId>(id).map_err(|e| e.to_string())
        }).and_then(|cid| {
            navigate(&format!("/chats/{}", cid.0));
            Ok(())
        }) {
            wasm_log!("{}", e);
    }
}


#[derive(Prop)]
struct ChatAppProps {
    id: String,
}

#[component]
fn ChatApp<G: Html>(ctx: Scope, sub: ChatAppProps) -> View<G> {
    let conversations: &Signal<Vec<ConversationId>> = create_signal(ctx, vec![]);
    provide_context_ref(ctx, conversations);

    let conversations_loaded = create_signal(ctx, false);
    provide_context_ref(ctx, conversations_loaded);

    let current_id: &Signal<Option<ConversationId>> = create_signal(ctx, None);
    provide_context_ref(ctx, current_id);

    let request_new_conversation = create_signal(ctx, None);
    provide_context_ref(ctx, request_new_conversation);

    let system_hint = create_signal(ctx, "".to_string());

    create_effect(ctx, move || {
        request_new_conversation.track();

        if !conversations_loaded.get_untracked().as_ref() {
            return;
        }

        if request_new_conversation.get().is_none() {
            return
        }

        sycamore::futures::spawn_local_scoped(ctx, async move {
            start_conversation(system_hint.get_untracked().as_ref().clone()).await;
        });
    });

    sycamore::futures::spawn_local_scoped(ctx, async move {
        match openai_get_conversations().await {
            Ok(list) => {
                match serde_wasm_bindgen::from_value::<Vec<ConversationId>>(list) {
                    Ok(list) => {
                        wasm_log!("conversations loaded: {:?}", list);
                        conversations.set(list); 
                        conversations_loaded.set(true);
                    },
                    Err(e) => {
                        wasm_log!("{:?}", e);
                    },
                }
            },
            Err(e) => {
                wasm_log!("{:?}", e);
            }
        };
    });

    if sub.id.is_empty() {
        view! {ctx,
            div(class="flex-1 flex flex-row") {
                ChatList {}
                NewChatGuide(prompt=system_hint, request_new=request_new_conversation)
            }
        }
    } else {
        view! {ctx,
            div(class="flex-1 flex flex-row") {
                ChatList {}
                ChatCompletion(id=sub.id.clone())
            }
        }
    }

}

#[component(inline_props)]
fn PromptItem<'a, G: Html>(ctx: Scope<'a>, prompt: Prompt, used: &'a Signal<String>) -> View<G> {
    let c = prompt.content.clone();

    view! {ctx,
        tr(class="w-full") {
            td { button(class="btn btn-info btn-outline btn-sm", on:click=move |_| {
                used.set(c.clone());
            }) { "use" } }
            td { (prompt.act) }
            td(class="w-full h-full block") {
                p(class="text-ellipsis overflow-hidden break-all", style="width: 400px") {
                    (prompt.content) 
                }
            }
        }
    }
}

#[derive(Prop)]
struct TextAreaProps<'a> {
    content: &'a Signal<String>,
    request_new: &'a Signal<Option<()>>,
    placeholder: String,
}

#[component]
fn TextArea<'a, G: Html>(ctx: Scope<'a>, props: TextAreaProps<'a>) -> View<G> {
    let on_click = |e: web_sys::Event| {
        e.prevent_default();
        props.request_new.set(Some(()));
        wasm_log!("request_new clicked");
    };

    view! {ctx,
        div(class="relative mb-2") {
            div(class="absolute bottom-2 right-2") {
                button(class="btn", on:click=on_click) {
                    svg(xmlns="http://www.w3.org/2000/svg",
                        fill="none",
                        viewBox="0 0 20 20",
                        stroke-width="1.0",
                        stroke="currentColor",
                        class="w-5 h-5") {
                        path(stroke-linecap="round",
                            stroke-linejoin="round",
                            d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5")
                    }
                }
            }

            textarea(class="w-full textarea textarea-info",
                rows=4,
                placeholder=props.placeholder,
                bind:value=props.content)
        }
    }
}

//TODO: suggesting prompts while typing
#[component(inline_props)]
fn NewChatGuide<'a, G: Html>(ctx: Scope<'a>, prompt: &'a Signal<String>, request_new: &'a Signal<Option<()>>) -> View<G> {
    let selected = create_signal(ctx, "".to_string());
    let preset_prompts = create_signal(ctx, vec![]);

    create_effect(ctx, move || {
        if selected.get().is_empty() {
            return;
        }

        sycamore::futures::spawn_local_scoped(ctx, async move {
            prompt.set((*selected.get_untracked()).clone());
        });
    });

    sycamore::futures::spawn_local_scoped(ctx, async move {
        if let Ok(prompts) = openai_bundled_prompts().await {
            if let Ok(prompts) = serde_wasm_bindgen::from_value::<Vec<Prompt>>(prompts) {
                preset_prompts.set(prompts);
            }
        }
    });

    view! {ctx,
        div(class="flex flex-col h-full w-full") {
            table(class="flex-1 table table-fixed border-spacing-0 boder-collapse overflow-hidden") {
                thead(class="w-full block") {
                    tr(class="flex w-full") {
                        th(class="w-1/6") { } 
                        th(class="w-1/6") { "Act" }
                        th(class="flex-1") { "Content" }
                    }
                }

                tbody(class="block overflow-y-auto w-full", style="height: calc(100vh - 300px)") {
                    Keyed(iterable=preset_prompts,
                        view=move |cx, x| {
                            view!{cx, PromptItem(prompt=x, used=selected)}
                        },
                        key=|x| x.act.clone())
                }
            }

            TextArea(placeholder="choose a system prompt or type your own...".to_string(), content=prompt, request_new=request_new)
        }
    }
}

#[component]
fn ChatList<G: Html>(ctx: Scope) -> View<G> {
    let conversations = use_context::<Signal<Vec<ConversationId>>>(ctx);
    let current_id = use_context::<Signal<Option<ConversationId>>>(ctx);

    view! { ctx,
        div(class="h-full flex flex-col mr-2 min-w-fit") {
            div(class="title shrink flex flex-row") {
                h2(class="shrink"){"Conversations"} 
            }

            ul(class="flex-1 flex flex-col my-2 overflow-y-scroll menu w-40 truncate") {
                Keyed(iterable=conversations,
                    view=|cx, x| {
                        if *current_id.get() == Some(x) {
                            view!(cx, 
                                li(class="hover-bordered"){
                                    a(class="active", href=format!("/chats/{}", x.0)){(x.0)}
                                })

                        } else {
                            view!(cx, 
                                li(class="hover-bordered"){
                                    a(href=format!("/chats/{}", x.0)){(x.0)}
                                })

                        }
                    },
                    key=|x| x.clone())
            }

            div(class="flex justify-center my-1") {
                button(class="btn btn-success btn-circle", on:click=|_| {
                    navigate("/chats");
                }) {
                    svg(xmlns="http://www.w3.org/2000/svg",viewBox="0 0 24 24",fill="currentColor",class="w-6 h-6") {
                        path(fill-rule="evenodd",
                            d="M12 3.75a.75.75 0 01.75.75v6.75h6.75a.75.75 0 010 1.5h-6.75v6.75a.75.75 0 01-1.5 0v-6.75H4.5a.75.75 0 010-1.5h6.75V4.5a.75.75 0 01.75-.75z",
                            clip-rule="evenodd")
                    }
                }
            }
        }
    }
}

async fn continue_conversation<'a>(conversation: &Signal<Conversation<'a>>, question: &Signal<String>) {
    let cnv = conversation.get_untracked();

    cnv.chats.modify().push(Message::new_user({
        let q = question.get().to_string();
        if q.is_empty() {
            return;
        }
        q
    }));

    question.set("".to_string());
    let prompt = cnv.chats.get();
    cnv.chats.modify().push(Message::new_assistant("...".to_string()));

    match serde_wasm_bindgen::to_value(prompt.as_ref()) {
        Ok(prompt) => {
            let id = serde_wasm_bindgen::to_value(cnv.id.get_untracked().as_ref()).unwrap();
            let msg: Message = match openai_completion(id, prompt).await {
                Ok(msg) => serde_wasm_bindgen::from_value(msg).unwrap(),
                Err(e) => {
                    wasm_log!("{:?}", e);
                    cnv.chats.modify().pop();
                    return;
                }
            };
            if let Some(p) = cnv.chats.modify().last_mut() {
                assert!(p.role == msg.role);
                p.content = msg.content;
            }

            highlightAll();
        },
        Err(e) => {
            wasm_log!("{:?}", e);
            cnv.chats.modify().pop();
        }
    }
}

async fn load_conversation<'a>(cid: ConversationId, conversation: &Signal<Conversation<'a>>) {
    wasm_log!("load conversation {:?}", cid);

    conversation.get_untracked().id.set(Some(cid));

    //serde_wasm_bindgen::to_value(&cid)
        //.map_err(|e| e.to_string())
        //.and_then(|id| {
            //openai_get_conversation(id).await.map_err(|e| format!("{:?}", e))
        //}).and_then(|msgs| {
            //serde_wasm_bindgen::from_value(msgs).map_err(|e| e.to_string())
        //}).and_then(|msgs: Vec<Message>| {
            //conversation.get_untracked().chats.set(msgs);
            //highlightAll();
        //});

    //return; 

    let id = match serde_wasm_bindgen::to_value(&cid) {
        Ok(id) => id,
        Err(e) => {
            wasm_log!("{:?}", e);
            return;
        }
    };

    let msgs = match openai_get_conversation(id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            wasm_log!("{:?}", e);
            return;
        }
    };
    //wasm_log!("{:?}", msgs);
    let msgs: Vec<Message> = serde_wasm_bindgen::from_value(msgs).unwrap();
    conversation.get_untracked().chats.set(msgs);
    highlightAll();
}

#[component]
fn ChatCompletion<G: Html>(ctx: Scope, props: ChatAppProps) -> View<G> {
    let question = create_signal(ctx, "".to_string());
    let clicked = create_signal(ctx, ());
    let waiting_for_response = create_signal(ctx, false);
    let submit_state = create_memo(ctx, || {
        if *waiting_for_response.get() {
            "btn btn-outline btn-info loading btn-sm btn-disabled"
        } else {
            "btn btn-outline btn-info btn-sm"
        }
    });

    let conversation = create_signal(ctx, Conversation::new(ctx));

    let current_id = use_context::<Signal<Option<ConversationId>>>(ctx);
    create_effect(ctx, move || {
        current_id.set(*conversation.get_untracked().id.get());
        wasm_log!("current_id changed to {:?}", current_id.get_untracked());
    });

    create_effect(ctx, move || {
        clicked.track();
        conversation.track();

        sycamore::futures::spawn_local_scoped(ctx, async move {
            let cnv = conversation.get_untracked();
            if cnv.id.get().is_none() {
                return;
            }

            waiting_for_response.set(true);
            continue_conversation(conversation, question).await;
            waiting_for_response.set(false);

            if cnv.title.get_untracked().is_empty() {
                // suggest one
                let id = serde_wasm_bindgen::to_value(cnv.id.get_untracked().as_ref()).unwrap();
                match openai_suggest_title(id).await {
                    Ok(title) => match serde_wasm_bindgen::from_value(title) {
                        Ok(title) => cnv.title.set(title),
                        Err(e) => wasm_log!("{:?}", e),
                    },
                    Err(e) => wasm_log!("{:?}", e),
                }
            }
        });
    });


    let id = props.id.clone();
    if !id.is_empty() {
        sycamore::futures::spawn_local_scoped(ctx, async move {
            let cid = ConversationId(Uuid::parse_str(&id).expect("uuid"));
            load_conversation(cid, conversation).await;
        });
    }

    view! { ctx,
        div(class="flex-1 h-full flex flex-col") {
            div(class="title shrink flex flex-row") {
                label(class="flex-1 badge badge-outline badge-info mb-2",
                    placeholder="context prompt") {
                    (conversation.get().title.get())
                }
            }

            ul(class="flex-1 flex flex-col my-2 overflow-y-scroll") {
                Keyed(iterable=conversation.get().chats,
                view=|cx, x| {
                    match x.role {
                        v if v == <KnownRoles as Into<&str>>::into(KnownRoles::Assistant) => view! {cx,
                        Bubble(actor="AI".to_string(),
                        at_start=false,
                        content=x.content)
                        },
                        v if v == <KnownRoles as Into<&str>>::into(KnownRoles::User) => view! {cx,
                            Bubble(actor="H".to_string(),
                            at_start=true,
                            content=x.content)
                        },
                        _ => view! {cx, }
                    }
                },
                key=|x| x.clone())
            }

            div(class="relative mb-2") {
                div(class="absolute bottom-2 right-2") {
                    button(class=*submit_state.get(),
                        on:click=|_| {
                        clicked.set(());
                    }) {
                        svg(xmlns="http://www.w3.org/2000/svg",
                            fill="none",
                            viewBox="0 0 20 20",
                            stroke-width="1.0",
                            stroke="currentColor",
                            class="w-5 h-5") {
                            path(stroke-linecap="round",
                                stroke-linejoin="round",
                                d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5")
                        }
                    }
                }
                textarea(class="textarea textarea-info w-full",
                    rows=4,
                    placeholder="ask your question...",
                    bind:value=question)
            }
        }
    }
}

#[derive(Prop)]
struct HomeItemProp<'a, G: Html> {
    link: &'a str,
    msg: &'a str,
    children: Children<'a, G>,
}

#[component]
fn HomeItem<'a, G: Html>(ctx: Scope<'a>, props: HomeItemProp<'a, G>) -> View<G> {
    let icon = props.children.call(ctx);
    let text = props.msg.to_owned();
    view! { ctx,
    div(class="card w-48 h-48 bg-primary text-primary-content") {
        div(class="card-body items-center text-center") {
            h2(class="card-title") {
                (icon)
            }
            div(class="card-actions justify-end mt-8") {
                a(class="btn",href=props.link) {
                    (text)
                }
            }
        }
    }
    }
}

#[component]
fn Home<G: Html>(ctx: Scope) -> View<G> {
    view! { ctx,
        div(class="flex-1 flex flex-row items-center justify-evenly") {
            HomeItem(link="/chats", msg="Chats") {
                svg(xmlns="http://www.w3.org/2000/svg",viewBox="0 0 24 24",fill="currentColor",class="w-6 h-6") {
                    path(fill-rule="evenodd",
                        d="M4.848 2.771A49.144 49.144 0 0112 2.25c2.43 0 4.817.178 7.152.52 1.978.292 3.348 2.024 3.348 3.97v6.02c0 1.946-1.37 3.678-3.348 3.97a48.901 48.901 0 01-3.476.383.39.39 0 00-.297.17l-2.755 4.133a.75.75 0 01-1.248 0l-2.755-4.133a.39.39 0 00-.297-.17 48.9 48.9 0 01-3.476-.384c-1.978-.29-3.348-2.024-3.348-3.97V6.741c0-1.946 1.37-3.68 3.348-3.97zM6.75 8.25a.75.75 0 01.75-.75h9a.75.75 0 010 1.5h-9a.75.75 0 01-.75-.75zm.75 2.25a.75.75 0 000 1.5H12a.75.75 0 000-1.5H7.5z",
                        clip-rule="evenodd")
                }
            }
            HomeItem(link="/voice", msg="voice") {
                svg(xmlns="http://www.w3.org/2000/svg",viewBox="0 0 24 24",fill="currentColor",class="w-6 h-6") {
                    path(d="M13.5 4.06c0-1.336-1.616-2.005-2.56-1.06l-4.5 4.5H4.508c-1.141 0-2.318.664-2.66 1.905A9.76 9.76 0 001.5 12c0 .898.121 1.768.35 2.595.341 1.24 1.518 1.905 2.659 1.905h1.93l4.5 4.5c.945.945 2.561.276 2.561-1.06V4.06zM18.584 5.106a.75.75 0 011.06 0c3.808 3.807 3.808 9.98 0 13.788a.75.75 0 11-1.06-1.06 8.25 8.25 0 000-11.668.75.75 0 010-1.06z")
                    path(d="M15.932 7.757a.75.75 0 011.061 0 6 6 0 010 8.486.75.75 0 01-1.06-1.061 4.5 4.5 0 000-6.364.75.75 0 010-1.06z")
                }
            }

            HomeItem(link="/codeassist", msg="Coding") {
                svg(xmlns="http://www.w3.org/2000/svg",viewBox="0 0 24 24",fill="currentColor",class="w-6 h-6") {
                    path(fill-rule="evenodd",
                        d="M2.25 6a3 3 0 013-3h13.5a3 3 0 013 3v12a3 3 0 01-3 3H5.25a3 3 0 01-3-3V6zm3.97.97a.75.75 0 011.06 0l2.25 2.25a.75.75 0 010 1.06l-2.25 2.25a.75.75 0 01-1.06-1.06l1.72-1.72-1.72-1.72a.75.75 0 010-1.06zm4.28 4.28a.75.75 0 000 1.5h3a.75.75 0 000-1.5h-3z",
                        clip-rule="evenodd" )
                }
            }
        }
    }
}

#[component]
fn Header<G: Html>(ctx: Scope) -> View<G> {
    view! { ctx,
    div(class="navbar bg-base-200") {
        div(class="navbar-start") {
            div(class="dropdown") {
                label(tabindex="0", class="btn btn-ghost bg-red-200 btn-circle") {
                    "M"
                }
                ul(tabindex="0", class="menu menu-compat dropdown-content mt-3 p-2 shadow rounded-md bg-base-100") {
                    li{a(href="/"){"Home"}}
                    li{a(href="/chats"){"Chats"}}
                    li{a(href="/codeassist"){"CodeAssist"}}
                    li{a(href="/game"){"Game"}}
                    li{a(href="/about"){"About"}}
                }
            }
        }

        div(class="navbar-center") {
            "Toolbox"
        }

        div(class="navbar-end") {
            a(class="btn btn-info normal-case text-xl", href="openai.com") { "openai" }
        }
    }

    }
}


fn window_event_listener<'a, F>(ctx: Scope<'a>, ev: &str, f: F)
where
    F: FnMut() + 'a,
{
    let boxed: Box<dyn FnMut()> = Box::new(f);
    let handler: Box<dyn FnMut() + 'static> = unsafe { std::mem::transmute(boxed) };
    let closure = create_ref(ctx, Closure::wrap(handler));

    let window = window().unwrap();
    window
        .add_event_listener_with_callback(ev, closure.as_ref().unchecked_ref())
        .unwrap_throw();
    on_cleanup(ctx, move || drop(closure));
}

#[component]
fn App<G: Html>(ctx: Scope) -> View<G> {
    window_event_listener(ctx, "load", || {
        wasm_log!("loaded");
        navigate("/");
    });

    window_event_listener(ctx, "resize", || {
        wasm_log!("resized");
    });

    view! { ctx,
        div(class="h-screen bg-base-100 flex flex-col overflow-hidden") {
            Header()

            div(class="flex-1 flex flex-row overflow-y-auto h-full") {
                Router(integration=HistoryIntegration::new(),
                    view=|cx, route: &ReadSignal<AppRoutes>| {
                        wasm_log!("route - {:?}", route.get());

                        view! { cx,
                            div(class="app flex-1 flex m-4") {
                                (match route.get().as_ref() {
                                    AppRoutes::ChatApp {id} => {
                                        view!{cx, ChatApp(id=id.first().map(|r|r.clone()).unwrap_or("".to_string()))}
                                    },
                                    AppRoutes::About => view!{cx, About},
                                    AppRoutes::Home => view!{cx, Home},
                                    AppRoutes::TextGame => view!{cx, NotFound},
                                    AppRoutes::CodeAssist => view!{cx, NotFound},
                                    AppRoutes::NotFound => view!{cx, NotFound},
                                })
                            }
                        }
                    }
                )

            }

        }
    }
}

fn main() {
    console_error_panic_hook::set_once();
    use tracing_wasm::*;
    let config = WASMLayerConfigBuilder::default()
        .set_max_level(tracing::Level::TRACE)
        .set_console_config(ConsoleConfig::ReportWithConsoleColor)
        .build();
    set_as_global_default_with_config(config);
    debug!("start");

    sycamore::render(|ctx| {
        view! { ctx,
            App
        }
    })
}

#[wasm_bindgen(module = "/api.js")]
extern "C" {
    #[wasm_bindgen(js_name = invokeCompletion, catch)]
    async fn openai_completion(id: JsValue, messages: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeStartConversation, catch)]
    async fn openai_start_conversation(hint: Option<String>) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeGetConversations, catch)]
    async fn openai_get_conversations() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeGetConversation, catch)]
    async fn openai_get_conversation(id: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeGetTitle, catch)]
    async fn openai_get_title(id: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeSetTitle, catch)]
    async fn openai_set_title(id: JsValue, title: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeSuggestTitle, catch)]
    async fn openai_suggest_title(id: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = invokeBundledPrompts, catch)]
    async fn openai_bundled_prompts() -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = hljs)]
    fn highlightAll();
    fn highlightAuto(html: JsValue) -> JsValue;
}

