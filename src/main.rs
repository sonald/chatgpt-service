use sycamore::prelude::*;
use tracing::{info, debug};
use wasm_bindgen::prelude::*;
use sycamore_router::{Route, Router, HistoryIntegration, navigate};
use web_sys::{window, console};

#[derive(Route)]
enum AppRoutes {
    #[to("/")]
    Home,
    #[to("/about")]
    About,
    #[to("/chat")]
    Completion,
    #[not_found]
    NotFound
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ConversationRound {
    human: String,
    ai: String,
}

#[derive(Debug, Clone)]
struct Conversation<'a> {
    topic: &'a Signal<String>,
    chats: &'a Signal<Vec<ConversationRound>>,
}

impl<'a> Conversation<'a> {
    pub fn new(ctx: Scope<'a>) -> Self {
        Conversation { 
            topic: create_signal(ctx, "".to_string()),
            chats: create_signal(ctx, Vec::default()),
        }
    }

    pub fn build_prompt(&self) -> String {
        let mut p = String::new();
        p.push_str(&format!("start a conversation about {}\n", self.topic));
        for r in &*self.chats.get() {
            p.push_str(&format!("Q: {}\n", r.human));
            p.push_str(&format!("A: {}\n", r.ai));
        }

        p
    }
}

//#[derive(Debug)]
//struct Conversations {
    //data: Vec<Conversation>,
//}

//impl Conversations {
    //pub fn new() -> Self {
        //Conversations { data: Vec::default() }
    //}
//}

#[derive(Prop)]
struct BubbleProps {
    actor: String,
    at_start: bool,
    content: String,
}

#[component]
fn Bubble<G: Html>(ctx: Scope, props: BubbleProps) -> View<G> {
    view! {
        ctx,
        div(class=(if props.at_start {"chat chat-start"} else {"chat chat-end"})) {
            div(class="chat-image avatar") {
                label(class="btn btn-circle rounded-full bg-slate-200") { (props.actor) }
            }
            div(class=(if props.at_start {"chat-bubble chat-bubble-secondary"} else {"chat-bubble chat-bubble-success"})) {
                (props.content)
            }
        }
    }
}

#[component]
fn Completion<G: Html>(ctx: Scope) -> View<G> {
    let question = create_signal(ctx, "".to_string());
    let clicked = create_signal(ctx, ());

    let conversation = create_signal(ctx, Conversation::new(ctx));

    conversation.modify().topic.set("knowledge".to_string());

    create_effect(ctx, move || {
        clicked.track();
        sycamore::futures::spawn_local_scoped(ctx, async move {
            let q = question.get().to_string();
            if q.is_empty() { return }
            conversation.get().chats.modify().push(ConversationRound {
                human: q,
                ai: "....".to_string()
            });

            question.set("".to_string());
            let prompt = conversation.get().build_prompt();
            let result = openai_completion(&prompt).await;
            if let Some(p) = conversation.get().chats.modify().last_mut() {
                p.ai = result.as_string().unwrap();
            }
        });
    });

    view! { ctx,
        div(class="flex-1 h-full flex flex-col") {
            h1(class="title shrink") {
                "Conversation: " (conversation.get().topic.get().to_string())
            } 

            ul(class="flex-1 flex flex-col my-2 max-h-5/6 overflow-y-scroll") {
                Keyed(iterable=conversation.get().chats,
                    view=|cx, x| view! {cx,
                        Bubble(actor="H".to_string(),
                                at_start=true, 
                                content=x.human)
                        Bubble(actor="AI".to_string(),
                                at_start=false, 
                                content=x.ai)
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
                ul(tabindex="0", class="menu menu-compat dropdown-content mt-3 p-2 shadow rounded-md bg-slate-700") {
                    li{a(href="/"){"Home"}}
                    li{a(href="/chat"){"Chat"}}
                    li{a(href="/about"){"About"}}
                }
            }
        }

        div(class="navbar-center") {
            "AI"
        }

        div(class="navbar-end") {
            a(class="btn btn-info normal-case text-xl", href="openai.com") { "openai" }
        }
    }

    }
}

#[component]
fn Side<G: Html>(ctx: Scope) -> View<G> {
    view! {
        ctx,
        div(class="drawer-mobile h-full w-40 shrink") {
            div(class="drawer-side") {
                ul(class="menu h-full p-4 w-40 bg-slate-700 text-base-content") {
                    li{a(href="/"){"Home"}}
                    li{a(href="/chat"){"Chat"}}
                    li{a(href="/about"){"About"}}
                }
            }
        }
    }
}

fn window_event_listener<'a, F>(ctx: Scope<'a>, ev: &str, f: F) where F: FnMut() + 'a {
    let boxed: Box<dyn FnMut()> = Box::new(f);
    let handler: Box<dyn FnMut() + 'static> = unsafe { std::mem::transmute(boxed) };
    let closure = create_ref(ctx, Closure::wrap(handler));

    let window = window().unwrap();
    window.add_event_listener_with_callback(ev, closure.as_ref().unchecked_ref()).unwrap_throw();
    on_cleanup(ctx, move || drop(closure));
}

#[component]
fn App<G: Html>(ctx: Scope) -> View<G> {

    //provide_context(ctx, Conversations::new());

    window_event_listener(ctx, "load", || {
        console::log_1(&"loaded".into()); 
        navigate("/chat");
    });

    window_event_listener(ctx, "resize", || {
        debug!("on load");
        console::log_1(&"resized".into()); 
        //navigate("/chat");
    });

    view! { ctx,
        div(class="h-screen bg-base-100 flex flex-col overflow-hidden") {
            Header()

            div(class="flex-1 flex flex-row overflow-y-auto h-full") {
                Side()

                Router(integration=HistoryIntegration::new(), 
                    view=|cx, route: &ReadSignal<AppRoutes>| {
                        view! { cx,
                            div(class="app flex-1 flex m-4") {
                                (match route.get().as_ref() {
                                    AppRoutes::Completion => view!{cx, Completion},
                                    AppRoutes::About => view!{cx, About},
                                    AppRoutes::Home => view!{cx, Home},
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
    #[wasm_bindgen(js_name = invokeCompletion)]
    async fn openai_completion(prompt: &str) -> JsValue;
}
