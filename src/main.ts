import "./app.css";
import App from "./App.svelte";
import Overlay from "./Overlay.svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";

const bootstrap = (): void => {
  window.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });

  let RootComponent = App;

  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    const currentWindow = getCurrentWindow();
    if (currentWindow.label === "overlay") {
      RootComponent = Overlay;
    }
  }

  new RootComponent({
    target: document.getElementById("app") as HTMLElement
  });
};

bootstrap();
