const invoke = window.__TAURI__.tauri.invoke;

export async function invokeCompletion(prompt) {
    return await invoke("completion", {prompt});
}
