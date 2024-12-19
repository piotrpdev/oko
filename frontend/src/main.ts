import "./app.css";
import App from "./App.svelte";

// TODO: Write unit tests for components/routes

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
