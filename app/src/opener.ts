// Opens a web page in the default browser. The Rust side starts the browser: on Linux it must not
// inherit the environment of the AppImage.
import { invoke } from "@tauri-apps/api/core";

export const openUrl = (url: string) => invoke<void>("open_link", { url });
