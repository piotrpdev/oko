<script lang="ts">
  import { replace } from "svelte-spa-router";
  import { user } from "../lib/userStore";

  async function logout() {
    const response = await fetch("/api/logout");

    if (response.ok) {
      $user = null;
      replace("/login");
    } else {
      alert("Logout failed");
    }
  }
</script>

<main>
  <pre>
        {JSON.stringify($user, null, 2)}
    </pre>
  <div>
    <button on:click={logout}>Logout</button>
  </div>
</main>
