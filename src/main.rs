#![allow(non_snake_case)]

use sycamore::prelude::*;
use sycamore_router::{navigate, HistoryIntegration, Route, Router};
use wasm_bindgen::prelude::*;
use web_sys::{console, window};
use pulldown_cmark as md;
use uuid::Uuid;

use common::*;

//#[derive(Debug, Route)]
//enum ChatRoute {
    //#[to("/<id>")]
    //Chat {id: String},
    //#[to("/")]
    //Index,
    //#[not_found]
    //NotFound,
//}

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
    topic: &'a Signal<String>,
    chats: &'a Signal<Vec<Message>>,
}

impl<'a> Conversation<'a> {
    pub fn new(ctx: Scope<'a>) -> Self {
        Conversation {
            id: create_signal(ctx, None),
            topic: create_signal(ctx, "".to_string()),
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

    sycamore::futures::spawn_local_scoped(ctx, async move {
        match openai_get_conversations().await {
            Ok(list) => {
                console::log_2(&"conversations: ".into(), &list);
                match serde_wasm_bindgen::from_value::<Vec<ConversationId>>(list) {
                    Ok(list) => { conversations.set(list); },
                    Err(e) => {
                        console::log_1(&e.to_string().into());
                    },
                }
            },
            Err(e) => {
                console::log_1(&e);
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

    view! { ctx,
        div(class="h-full flex flex-col mr-2 min-w-fit") {
            div(class="title shrink flex flex-row") {
                h2(class="shrink"){"Conversations"} 
            }

            ul(class="flex-1 flex flex-col my-2 overflow-y-scroll menu w-40 truncate") {
                Keyed(iterable=conversations,
                view=|cx, x| {
                    view!{cx, 
                        li(class="truncate"){
                            a(href=format!("/chats/{}", x.0)){(x.0)}
                        }
                    }
                },
                key=|x| x.clone())
            }

            div(class="flex justify-center my-1") {
                button(class="btn btn-success btn-circle", on:click=|_| {
                    console::log_1(&"new clicked".into());
                    navigate("/chats/");
                }) {
                    "+"
                }
            }
        }
    }
}

#[component]
fn ChatCompletion<G: Html>(ctx: Scope, props: ChatAppProps) -> View<G> {
    let question = create_signal(ctx, "".to_string());
    let clicked = create_signal(ctx, ());

    let conversation = create_signal(ctx, Conversation::new(ctx));
    let conversations = use_context::<Signal<Vec<ConversationId>>>(ctx);

    console::log_1(&"enter ChatCompletion".into());

    create_effect(ctx, move || {
        clicked.track();
        conversation.track();
        sycamore::futures::spawn_local_scoped(ctx, async move {
            if conversation.get().id.get().is_none() {
                return;
            }

            conversation.get().chats.modify().push(Message::new_user({
                let q = question.get().to_string();
                if q.is_empty() {
                    return;
                }
                q
            }));

            question.set("".to_string());
            let prompt = conversation.get().chats.get();
            conversation.get().chats.modify().push(Message::new_assistant("...".to_string()));

            match serde_wasm_bindgen::to_value(prompt.as_ref()) {
                Ok(prompt) => {
                    let id = serde_wasm_bindgen::to_value(conversation.get().id.get().as_ref()).unwrap();
                    let msg: Message = match openai_completion(id, prompt).await {
                        Ok(msg) => serde_wasm_bindgen::from_value(msg).unwrap(),
                        Err(e) => {
                            console::log_1(&e);
                            conversation.get().chats.modify().pop();
                            return;
                        }
                    };
                    if let Some(p) = conversation.get().chats.modify().last_mut() {
                        assert!(p.role == msg.role);
                        p.content = msg.content;
                    }

                    highlightAll();
                },
                Err(e) => {
                    console::log_1(&e.to_string().into());
                    conversation.get().chats.modify().pop();
                }
            }
        });
    });

    let id = props.id.clone();
    sycamore::futures::spawn_local_scoped(ctx, async move {
        if id.len() > 0 {
            let cid = ConversationId(Uuid::parse_str(&id).expect("uuid"));
            conversation.get().id.set(Some(cid));
            conversation.modify().topic.set("you are a software engineer".to_string());

            let id = match serde_wasm_bindgen::to_value(&cid) {
                Ok(id) => id,
                Err(e) => {
                    console::log_1(&e.to_string().into());
                    return;
                }
            };

            let msgs = openai_get_conversation(id).await.unwrap();
            //console::log_1(&msgs);
            let msgs: Vec<Message> = serde_wasm_bindgen::from_value(msgs).unwrap();
            conversation.get().chats.set(msgs);
            highlightAll();

            if conversation.get().id.get().is_some() { 
                return;
            }

            return
        }

        if conversations.get().len() > 0 {
            console::log_1(&"load existing conversation".into());
            return;
        }

        console::log_1(&"start conversation".into());
        match openai_start_conversation().await {
            Ok(id) => {
                console::log_2(&"created:".into(), &id);
                match serde_wasm_bindgen::from_value(id) {
                    Ok(id) => {
                        conversation.get().id.set(Some(id));
                        conversation.modify().topic.set("you are a software engineer".to_string());
                    },
                    Err(e) => {
                        console::log_1(&e.to_string().into());
                    },
                }
            }
            Err(e) => {
                console::log_1(&e);
                return;
            }
        };
    });

    view! { ctx,
        div(class="flex-1 h-full flex flex-col") {
            div(class="title shrink flex flex-row") {
                h1(class="shrink"){"Conversation: "} 
                label(class="flex-1 badge badge-outline badge-info mb-2",
                    placeholder="context prompt") {
                    (conversation.get().topic.get())
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

            textarea(class="textarea textarea-info mb-2", placeholder="type here", bind:value=question)

            button(class="btn btn-info", on:click=|_| {
                console::log_1(&"clicked".into());
                clicked.set(());
            }) {
                "talk"
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
        console::log_1(&"loaded".into());
        navigate("/chats");
    });

    window_event_listener(ctx, "resize", || {
        console::log_1(&"resized".into());
    });

    view! { ctx,
        div(class="h-screen bg-base-100 flex flex-col overflow-hidden") {
            Header()

            div(class="flex-1 flex flex-row overflow-y-auto h-full") {
                Router(integration=HistoryIntegration::new(),
                    view=|cx, route: &ReadSignal<AppRoutes>| {
                        console::log_1(&format!("route - {:?}", route.get()).into());

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
    tracing_wasm::set_as_global_default();

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
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = hljs)]
    fn highlightAll();
    fn highlightAuto(html: JsValue) -> JsValue;
}
