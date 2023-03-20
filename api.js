const invoke = window.__TAURI__.tauri.invoke;

export async function invokeCompletion(id, messages) {
    return await invoke("completion", {id, messages});
}

export async function invokeStartConversation(hint) {
    return await invoke("start_conversation", {hint});
}

export async function invokeGetConversations() {
    return await invoke("get_conversations");
}

export async function invokeGetConversation(id) {
    return await invoke("get_conversation", {id});
}

export async function invokeGetTitle(id) {
    return await invoke("get_title", {id});
}

export async function invokeSetTitle(id, title) {
    return await invoke("set_title", {id, title});
}

export async function invokeSuggestTitle(id) {
    return await invoke("suggest_title", {id});
}

export async function invokeBundledPrompts() {
    return await invoke("bundled_prompts");
}

export async function invokeGenerateImage(req) {
    return await invoke("generate_image", {req});
} 
