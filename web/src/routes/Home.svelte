<script lang="ts">
  import { replace } from "svelte-spa-router";
  import { user } from "../lib/userStore";
  import { onDestroy, onMount } from "svelte";

  let socket: WebSocket;
  let frameCount = 0;

  async function logout() {
    const response = await fetch("/api/logout");

    if (response.ok) {
      $user = null;
      replace("/login");
    } else {
      alert("Logout failed");
    }
  }

  function onMessage(event: MessageEvent) {
    // const data = JSON.parse(event.data);
    console.log(event);

    const data = event.data;

    if (data instanceof Blob) {
      console.log("Received image");
      frameCount++;
      const url = URL.createObjectURL(data);
      const img = document.querySelector("img");
      if (img) img.src = url;
    }
  }

  onMount(() => {
    socket = new WebSocket(`ws://${window.location.host}/api/ws`);
    socket.onmessage = onMessage;
  });

  onDestroy(() => {
    socket.close();
  });
</script>

<main>
  <div>
    <h3>Username</h3>
    <span>{$user?.user.username}</span>
  </div>
  <div id="live-feed">
    <h3>Live Feed</h3>
    <span>Frame: {frameCount}</span>
    <img width="800px" height="600px" alt="live feed" />
  </div>
  <div>
    <button id="logout" on:click={logout}>Logout</button>
  </div>
</main>

<style>
  #live-feed {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>
