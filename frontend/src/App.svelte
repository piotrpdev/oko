<script lang="ts">
  import "./app.css";
  import Router, { replace, location } from "svelte-spa-router";
  import PWABadge from "./lib/PWABadge.svelte";
  import Cameras from "./routes/Cameras.svelte";
  import Login from "./routes/Login.svelte";
  import wrap from "svelte-spa-router/wrap";
  import { onDestroy, onMount } from "svelte";
  import { socket } from "$lib/stores/socketStore";
  import Home from "./routes/Home.svelte";
  import NotFound from "./routes/NotFound.svelte";

  // TODO: Add transitions to everything
  // TODO: Replace console.error and log with toast notifications
  // TODO: Stop relying on default form values
  // TODO: Add 404 page

  const isAuthorized = () =>
    fetch("/api/").then((response) => {
      // Redirection occurs if not logged in
      if (response.redirected) {
        return false;
      }

      return true;
    });

  // TODO: Make these async
  const routes = {
    "/": wrap({
      component: Home,
      conditions: [isAuthorized],
    }),
    "/cameras": wrap({
      component: Cameras,
      conditions: [isAuthorized],
    }),
    "/login": Login,
    "*": NotFound,
  };

  function onOpen() {
    $socket?.send("user");
  }

  function closeSocket() {
    $socket?.removeEventListener("open", onOpen);
    $socket?.close();
    $socket = null;
  }

  $: (() => {
    if ($location === "/login") {
      closeSocket();
      return;
    }

    if ($socket != null) return;

    $socket = new WebSocket(`ws://${window.location.host}/api/ws`);
    $socket?.addEventListener("open", onOpen);
  })();

  onDestroy(() => closeSocket());
</script>

<Router {routes} on:conditionsFailed={() => replace("/login")} />

<PWABadge />
