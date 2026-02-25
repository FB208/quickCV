import "./app.css";
import App from "./App.svelte";
import Overlay from "./Overlay.svelte";

const bootstrap = async (): Promise<void> => {
  let RootComponent = App;

  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const currentWindow = getCurrentWindow();
    if (currentWindow.label === "overlay") {
      RootComponent = Overlay;
    }
  }

  new RootComponent({
    target: document.getElementById("app") as HTMLElement
  });
};

void bootstrap();
