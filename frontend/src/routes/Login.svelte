<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import { replace } from "svelte-spa-router";
  import { user } from "../lib/stores/userStore";
  import { onMount } from "svelte";

  const DEFAULT_ADMIN_USERNAME = "admin";
  const DEFAULT_ADMIN_PASSWORD = "hunter42";
  const DEFAULT_GUEST_USERNAME = "guest";
  const DEFAULT_GUEST_PASSWORD = "hunter42";

  let username = import.meta.env.DEV ? DEFAULT_ADMIN_USERNAME : "";
  let password = import.meta.env.DEV ? DEFAULT_ADMIN_PASSWORD : "";

  let guest_exists = false;

  async function handleSubmit() {
    const response = await fetch("/api/login", {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        username,
        password,
      }),
    });

    if (response.ok) {
      const response = await fetch("/api/");

      if (response.redirected) {
        console.error("You need to login first");
        return;
      }

      if (response.ok) {
        const data = await response.json();
        $user = data;
      } else {
        console.error("Failed to get data");
      }

      replace("/");
    } else {
      console.error("Login failed");
    }
  }

  onMount(async () => {
    const response = await fetch("/api/guest_exists");

    if (response.ok) {
      guest_exists = true;
    } else {
      console.error("Failed to check guest exists");
    }
  });
</script>

<div class="relative flex min-h-screen flex-col bg-background">
  <div class="theme-zinc flex h-screen w-full items-center justify-center px-4">
    <Card.Root class="w-full max-w-sm">
      <form on:submit|preventDefault={handleSubmit}>
        <Card.Header>
          <Card.Title class="text-2xl">Login</Card.Title>
          <Card.Description
            >Enter your email below to login to your account.</Card.Description
          >
        </Card.Header>
        <Card.Content class="grid gap-4">
          <div class="grid gap-2">
            <Label for="username">Email</Label>
            <Input
              name="username"
              id="username"
              placeholder="admin"
              required
              bind:value={username}
            />
          </div>
          <div class="grid gap-2">
            <Label for="password">Password</Label>
            <Input
              name="password"
              id="password"
              type="password"
              required
              bind:value={password}
            />
          </div>
        </Card.Content>
        <Card.Footer class="flex-col gap-4">
          <Button id="login" class="w-full" type="submit">Sign in</Button>
          {#if guest_exists}
            <Button
              id="login-guest"
              variant="outline"
              class="w-full"
              type="button"
              on:click={() => {
                username = DEFAULT_GUEST_USERNAME;
                password = DEFAULT_GUEST_PASSWORD;
                handleSubmit();
              }}
            >
              Sign in as Guest
            </Button>
          {/if}
        </Card.Footer>
      </form>
    </Card.Root>
  </div>
</div>
