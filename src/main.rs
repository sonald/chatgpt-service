#![allow(non_snake_case)]

use sycamore::prelude::*;
use sycamore_router::{navigate, HistoryIntegration, Route, Router};
use wasm_bindgen::prelude::*;
use web_sys::{console, window};
use pulldown_cmark as md;
use uuid::Uuid;
use tracing::{debug, info};

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

    let request_new_conversation = create_signal(ctx, ());
    provide_context_ref(ctx, request_new_conversation);

    sycamore::futures::spawn_local_scoped(ctx, async move {
        match openai_get_conversations().await {
            Ok(list) => {
                wasm_log!("conversations: {:?}", list);
                match serde_wasm_bindgen::from_value::<Vec<ConversationId>>(list) {
                    Ok(list) => {
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

    view! {ctx,
        div(class="flex-1 flex flex-row") {
            ChatList {}
            ChatCompletion(id=sub.id.clone())
        }
    }
}

#[component]
fn ChatList<G: Html>(ctx: Scope) -> View<G> {
    let conversations = use_context::<Signal<Vec<ConversationId>>>(ctx);
    let current_id = use_context::<Signal<Option<ConversationId>>>(ctx);
    let request_new_conversation = use_context::<Signal<()>>(ctx);

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
                    wasm_log!("new clicked");
                    request_new_conversation.set(());
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

async fn check_start_conversation<'a>(conversation: &Signal<Conversation<'a>>) {
    let cnv = conversation.get_untracked();
    match openai_start_conversation().await {
        Ok(id) => {
            wasm_log!("created: {:?}", id);
            match serde_wasm_bindgen::from_value(id) {
                Ok(cid) => {
                    cnv.id.set(Some(cid));
                    cnv.title.set("you are a software engineer".to_string());
                    cnv.chats.modify().clear();

                    navigate(&format!("/chats/{}", cid.0));
                },
                Err(e) => {
                    wasm_log!("{}", e.to_string());
                },
            }
        }
        Err(e) => {
            wasm_log!("{:?}", e);
            return;
        }
    };
}

async fn load_conversation<'a>(cid: ConversationId, conversation: &Signal<Conversation<'a>>) {
    wasm_log!("load conversation {:?}", cid);

    conversation.get_untracked().id.set(Some(cid));

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
            "btn btn-info btn-circle btn-disabled"
        } else {
            "btn btn-info btn-circle"
        }
    });

    let conversation = create_signal(ctx, Conversation::new(ctx));

    let conversations = use_context::<Signal<Vec<ConversationId>>>(ctx);
    let conversations_loaded = use_context::<Signal<bool>>(ctx);
    let request_new_conversation = use_context::<Signal<()>>(ctx);
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

    create_effect(ctx, move || {
        request_new_conversation.track();
        if !conversations_loaded.get_untracked().as_ref() {
            return;
        }

        sycamore::futures::spawn_local_scoped(ctx, async move {
            check_start_conversation(conversation).await;
        });
    });

    let id = props.id.clone();
    if id.is_empty() {
        // enter with empty state
        create_effect(ctx, move || {
            if !conversations_loaded.get().as_ref() {
                return;
            }
            sycamore::futures::spawn_local_scoped(ctx, async move {
                if conversations.get().is_empty() {
                    check_start_conversation(conversation).await;
                } else {
                    let id = conversations.get().first().unwrap().clone();
                    wasm_log!("load existing conversation {:?}", id);
                    load_conversation(id, conversation).await;
                }
            });
        });

    } else {
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

            ul(class="flex-1 flex flex-col my-2 max-h-5/6 overflow-y-scroll") {
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
                div(class="absolute top-3 right-2") {
                    button(class=*submit_state.get(),
                        on:click=|_| {
                        wasm_log!("clicked");
                        clicked.set(());
                    }) {
                        svg(xmlns="http://www.w3.org/2000/svg",viewBox="0 0 24 24",fill="currentColor",class="w-6 h-6") {
                            path(fill-rule="evenodd",
                                d="M4.5 5.653c0-1.426 1.529-2.33 2.779-1.643l11.54 6.348c1.295.712 1.295 2.573 0 3.285L7.28 19.991c-1.25.687-2.779-.217-2.779-1.643V5.653z",
                                clip-rule="evenodd")
                        }
                    }
                }
                textarea(class="textarea textarea-info w-full",
                    placeholder="ask your question...",
                    bind:value=question)
            }
        }
    }
}

#[component]
fn Home<G: Html>(ctx: Scope) -> View<G> {
    view! { ctx,
        div(class="card flex-1 items-center") {
            h1 { "Home" }
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
        navigate("/chats");
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
    async fn openai_start_conversation() -> Result<JsValue, JsValue>;
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
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = hljs)]
    fn highlightAll();
    fn highlightAuto(html: JsValue) -> JsValue;
}
