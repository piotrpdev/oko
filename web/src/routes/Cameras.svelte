<script lang="ts">
  import Trash from "lucide-svelte/icons/trash";
  import CirclePlus from "lucide-svelte/icons/circle-plus";
  import Download from "lucide-svelte/icons/download";
  import RotateCw from "lucide-svelte/icons/rotate-cw";

  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import { user } from "../lib/stores/userStore";
  import { onDestroy, onMount } from "svelte";
  import * as Table from "$lib/components/ui/table/index.js";
  import DashboardLayout from "$lib/layouts/DashboardLayout.svelte";
  import { socket } from "$lib/stores/socketStore";
  import {
    isImageContainer,
    type Camera,
    type VideoCameraView,
  } from "../types";

  let frameCount = 0;
  let imgSrc: string = "";

  let name = "Backyard";
  let address = "192.168.0.30";

  // TODO: Refresh cameras on add/remove
  // ? Maybe use a store for cameras
  // ? Maybe show confirmation dialog on remove

  let addCameraDialogOpen = false;
  let getCamerasPromise = getCameras();

  const refreshCameras = () => (getCamerasPromise = getCameras());

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

  async function addCamera() {
    const response = await fetch("/api/cameras", {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        name,
        address,
      }),
    });

    if (response.ok) {
      console.log("Camera added");
      addCameraDialogOpen = false;
      refreshCameras();
    } else {
      console.error("Add Camera failed");
    }
  }

  async function removeCamera(cameraId: number) {
    const response = await fetch(`/api/cameras/${cameraId}`, {
      method: "DELETE",
    });

    if (response.ok) {
      console.log("Camera removed");
      refreshCameras();
    } else {
      console.error("Remove Camera failed");
    }
  }

  let videosPromise = getVideos(2);

  const refreshVideos = () => (videosPromise = getVideos(2));

  async function getVideos(cameraId: number): Promise<VideoCameraView[]> {
    const response = await fetch(`/api/cameras/${cameraId}/videos`);

    if (response.ok) {
      const data = await response.json();
      return data;
    } else {
      console.error("Failed to fetch videos");
      throw new Error("Failed to fetch videos");
    }
  }

  function onMessage(event: MessageEvent) {
    const data = event.data;

    try {
      const parsed_msg = JSON.parse(data);

      if (isImageContainer(parsed_msg)) {
        console.log("Frame received");
        frameCount++;
        const bytes = new Uint8Array(parsed_msg.image_bytes);
        const blob = new Blob([bytes], { type: "image/jpeg" });
        const url = URL.createObjectURL(blob);
        imgSrc = url;
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

<DashboardLayout tab="Cameras">
  <div class="mx-auto grid w-full max-w-6xl gap-2">
    <h1 class="text-3xl font-semibold">Cameras</h1>
  </div>
  <div
    class="mx-auto grid w-full max-w-6xl items-start gap-6 md:grid-cols-[180px_1fr] lg:grid-cols-[250px_1fr]"
  >
    <nav
      class="grid gap-4 text-sm text-muted-foreground"
      data-x-chunk-container="chunk-container after:right-0"
    >
      {#await getCamerasPromise}
        <!-- TODO: Use skeletons -->
        <span class="px-3 py-0 text-muted-foreground">Loading...</span>
      {:then cameras}
        {#each cameras as camera}
          <div class="group flex items-center gap-3 rounded-lg px-3 py-0">
            <a
              href="##"
              data-camera-id={camera.camera_id}
              class={"text-muted-foreground transition-all hover:text-primary" +
                (camera.camera_name === "Kitchen"
                  ? " font-semibold text-primary"
                  : "")}>{camera.camera_name}</a
            >
            <Button
              on:click={() => removeCamera(camera.camera_id)}
              variant="ghost"
              size="icon"
              aria-label="Remove Camera"
              data-camera-id={camera.camera_id}
              class="ml-auto flex h-4 w-4 shrink-0 items-center justify-center opacity-0 transition-all group-hover:opacity-100"
            >
              <Trash class="h-4 w-4" />
            </Button>
          </div>
        {/each}
        {#if $user?.user?.username === "admin"}
          <Dialog.Root bind:open={addCameraDialogOpen}>
            <Dialog.Trigger
              id="add-camera"
              class={buttonVariants({ variant: "outline" }) + " gap-1"}
            >
              <CirclePlus class="h-3.5 w-3.5" />
              Add Camera
            </Dialog.Trigger>
            <Dialog.Content class="sm:max-w-[425px]">
              <form class="contents" on:submit|preventDefault={addCamera}>
                <Dialog.Header>
                  <Dialog.Title>Add Camera</Dialog.Title>
                </Dialog.Header>
                <div class="grid gap-4 py-4">
                  <div class="grid grid-cols-4 items-center gap-4">
                    <Label for="name" class="text-right">Name</Label>
                    <Input
                      id="name"
                      placeholder="Backyard"
                      bind:value={name}
                      class="col-span-3"
                    />
                  </div>
                  <div class="grid grid-cols-4 items-center gap-4">
                    <Label for="address" class="text-right">IP Address</Label>
                    <Input
                      id="address"
                      placeholder="192.168.0.30"
                      bind:value={address}
                      class="col-span-3"
                    />
                  </div>
                </div>
                <Dialog.Footer>
                  <Button type="submit">Submit</Button>
                </Dialog.Footer>
              </form>
            </Dialog.Content>
          </Dialog.Root>
        {/if}
      {:catch error}
        <p>{error.message}</p>
      {/await}
    </nav>
    <div class="grid gap-6">
      <Card.Root>
        <Card.Header>
          <Card.Title>Kitchen</Card.Title>
          <Card.Description>
            Frame: {frameCount}
          </Card.Description>
        </Card.Header>
        <Card.Content>
          <!-- TODO: Add placeholder image/skeleton -->
          <!-- TODO: Maybe use known camera resolution as aspect ratio when camera is offline -->
          <img
            id="live-feed"
            class="aspect-[4/3]"
            src={imgSrc}
            alt="live feed"
            style={`visibility: ${imgSrc ? "visible" : "hidden"}`}
          />
        </Card.Content>
      </Card.Root>
      <Card.Root>
        <Card.Header class="flex flex-row items-center">
          <Card.Title>Recordings</Card.Title>
          <Button
            on:click={() => refreshVideos()}
            aria-label="Refresh"
            variant="outline"
            size="icon"
            class="!mt-0 ml-auto"
          >
            <RotateCw class="h-4 w-4" />
          </Button>
        </Card.Header>
        <Card.Content>
          <Table.Root>
            <Table.Header>
              <Table.Row>
                <Table.Head>Name</Table.Head>
                <Table.Head>Download</Table.Head>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {#await videosPromise}
                <!-- TODO: Use skeletons -->
                <span class="px-3 py-0 text-muted-foreground">Loading...</span>
              {:then videos}
                {#each videos as video}
                  <Table.Row data-video-id={video.video_id}>
                    <Table.Cell class="font-semibold"
                      >{video.file_path.split("/").at(-1)}</Table.Cell
                    >
                    <Table.Cell>
                      <Button
                        variant="outline"
                        size="icon"
                        aria-label="Download"
                        data-video-id={video.video_id}
                        href={`/api/videos/${video.video_id}`}
                        download={video.file_path.split("/").at(-1)}
                      >
                        <Download class="h-4 w-4" />
                      </Button>
                    </Table.Cell>
                  </Table.Row>
                {/each}
              {:catch error}
                <p>{error.message}</p>
              {/await}
            </Table.Body>
          </Table.Root>
        </Card.Content>
      </Card.Root>
    </div>
  </div>
</DashboardLayout>
