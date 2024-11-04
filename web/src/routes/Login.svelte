<script lang="ts">
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

<main>
  <form on:submit|preventDefault={handleSubmit}>
    <fieldset>
      <legend>User login</legend>
      <p>
        <label for="username">Username</label>
        <input name="username" id="username" bind:value={username} />
      </p>
      <p>
        <label for="password">Password</label>
        <input
          name="password"
          id="password"
          type="password"
          bind:value={password}
        />
      </p>
    </fieldset>

    <button id="login" type="submit">Login</button>
  </form>
</main>
