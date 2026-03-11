import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { QueryProvider } from "./providers/QueryProvider";
import "./styles.css";

async function prepare() {
  if (import.meta.env.DEV) {
    const { worker } = await import("./mocks/browser");
    await worker.start({ onUnhandledRequest: "bypass" });
  }
}

prepare().then(() => {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryProvider>
        <App />
      </QueryProvider>
    </React.StrictMode>,
  );
});
