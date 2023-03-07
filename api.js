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
