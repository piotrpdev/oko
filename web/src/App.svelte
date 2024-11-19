<script lang="ts">
  import "./app.css";
  import Router, { replace } from "svelte-spa-router";
  import PWABadge from "./lib/PWABadge.svelte";
  import Home from "./routes/Home.svelte";
  import Login from "./routes/Login.svelte";
  import wrap from "svelte-spa-router/wrap";

  // TODO: Add transitions to everything
  // TODO: Replace console.error and log with toast notifications
  // TODO: Stop relying on default form values

  // TODO: Make these async
  const routes = {
    "/": wrap({
      component: Home,
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
</script>

<Router {routes} on:conditionsFailed={() => replace("/login")} />

<PWABadge />
