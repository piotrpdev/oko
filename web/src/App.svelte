<script lang="ts">
  import "./app.css";
  import Router, { replace } from "svelte-spa-router";
  import PWABadge from "./lib/PWABadge.svelte";
  import Cameras from "./routes/Cameras.svelte";
  import Login from "./routes/Login.svelte";
  import wrap from "svelte-spa-router/wrap";
  import { onDestroy, onMount } from "svelte";
  import { socket } from "$lib/stores/socketStore";

  // TODO: Add transitions to everything
  // TODO: Replace console.error and log with toast notifications
  // TODO: Stop relying on default form values
  // TODO: Add 404 page

  // TODO: Make these async
  const routes = {
    "/": wrap({
      component: Cameras,
      conditions: [() => false],
    }),
    "/cameras": wrap({
      component: Cameras,
      conditions: [
        () =>
          fetch("/api/").then((response) => {
            // Redirection occurs if not logged in
            if (response.redirected) {
              return false;
            }

            return true;
          }),
      ],
    }),
    "/login": Login,
  };

  function onOpen() {
    $socket?.send("user");
  }

  onMount(() => {
    $socket = new WebSocket(`ws://${window.location.host}/api/ws`);
    $socket?.addEventListener("open", onOpen);
  });

  onDestroy(() => {
    $socket?.removeEventListener("open", onOpen);
    $socket?.close();
  });
</script>

<Router {routes} on:conditionsFailed={() => replace("/login")} />

<PWABadge />
