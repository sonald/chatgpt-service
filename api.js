const invoke = window.__TAURI__.tauri.invoke;

export async function invokeCompletion(id, messages) {
    return await invoke("completion", {id, messages});
}

export async function invokeStartConversation() {
    return await invoke("start_conversation");
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
