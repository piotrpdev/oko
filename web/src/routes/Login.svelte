<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import { replace } from "svelte-spa-router";
  import { user } from "../lib/userStore";

  let username = "admin";
  let password = "hunter42";

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
        alert("You need to login first");
        return;
      }

      if (response.ok) {
        const data = await response.json();
        $user = data;
      } else {
        alert("Failed to get data");
      }

      replace("/");
    } else {
      alert("Login failed");
    }
  }
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
        <Card.Footer>
          <Button id="login" class="w-full" type="submit">Sign in</Button>
        </Card.Footer>
      </form>
    </Card.Root>
  </div>
</div>
