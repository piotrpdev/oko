<script lang="ts">
  import { socket } from "$lib/stores/socketStore";
  import DashboardLayout from "$lib/layouts/DashboardLayout.svelte";
  import * as Card from "$lib/components/ui/card/index.js";

  import { onDestroy, onMount } from "svelte";
  import { isImageContainer, type Camera } from "../types";

  let cameraSources: Record<string, string> = {};

  let getCamerasPromise = getCameras();

  //   const refreshCameras = () => (getCamerasPromise = getCameras());

  async function getCameras(): Promise<Camera[]> {
    const response = await fetch("/api/cameras");

    if (response.ok) {
      const data = await response.json();
      return data;
    } else {
      console.error("Failed to fetch cameras");
      throw new Error("Failed to fetch cameras");
    }
  }

  function onMessage(event: MessageEvent) {
    const data = event.data;

    try {
      const parsed_msg = JSON.parse(data);

      if (isImageContainer(parsed_msg)) {
        const bytes = new Uint8Array(parsed_msg.image_bytes);
        const blob = new Blob([bytes], { type: "image/jpeg" });
        const url = URL.createObjectURL(blob);
        cameraSources[parsed_msg.camera_id] = url;
      }
    } catch (e) {
      console.error("Failed to parse WebSocket message JSON");
    }
  }

  onMount(() => {
    $socket?.addEventListener("message", onMessage);
  });

  onDestroy(() => {
    $socket?.removeEventListener("message", onMessage);
  });
</script>

<DashboardLayout tab="Home">
  <div class="mx-auto grid w-full max-w-6xl gap-2">
    <h1 class="text-3xl font-semibold">Home</h1>
  </div>
  <!-- ? Maybe make this even wider? -->
  <div
    class="mx-auto grid w-full max-w-6xl items-start gap-6 md:grid-cols-2 lg:grid-cols-3"
  >
    {#await getCamerasPromise}
      <!-- TODO: Use skeletons -->
      <span class="px-3 py-0 text-muted-foreground">Loading...</span>
    {:then cameras}
      {#each cameras as camera}
        <Card.Root>
          <Card.Header>
            <Card.Title>{camera.camera_name}</Card.Title>
          </Card.Header>
          <Card.Content>
            <!-- TODO: Add placeholder image/skeleton -->
            <!-- TODO: Maybe use known camera resolution as aspect ratio when camera is offline -->
            <img
              id="live-feed"
              class="aspect-[4/3]"
              data-camera-id={camera.camera_id}
              src={cameraSources[camera.camera_id] || ""}
              alt="live feed"
              style={`visibility: ${cameraSources[camera.camera_id] ? "visible" : "hidden"}`}
            />
          </Card.Content>
        </Card.Root>
      {/each}
    {:catch error}
      <p>{error.message}</p>
    {/await}
  </div>
</DashboardLayout>
