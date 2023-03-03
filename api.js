const invoke = window.__TAURI__.tauri.invoke;

export async function invokeCompletion(messages) {
    return await invoke("completion", {messages});
}
