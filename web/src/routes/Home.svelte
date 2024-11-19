<script lang="ts">
  import CircleUser from "lucide-svelte/icons/circle-user";
  import Menu from "lucide-svelte/icons/menu";
  import Package2 from "lucide-svelte/icons/package-2";
  import Trash from "lucide-svelte/icons/trash";
  import CirclePlus from "lucide-svelte/icons/circle-plus";

  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import { replace } from "svelte-spa-router";
  import { user, type Camera } from "../lib/userStore";
  import { onDestroy, onMount } from "svelte";
  import { Badge } from "$lib/components/ui/badge";

  let socket: WebSocket;
  let frameCount = 0;
  let imgSrc: string = "";

  async function logout() {
    const response = await fetch("/api/logout");

    if (response.ok) {
      $user = null;
      replace("/login");
    } else {
      alert("Logout failed");
    }
  }

  // $user?.user.username

  function onMessage(event: MessageEvent) {
    const data = event.data;

    if (data instanceof Blob) {
      console.log("Frame received");
      console.log(data);
      frameCount++;
      const url = URL.createObjectURL(data);
      imgSrc = url;
    }
  }

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
      alert("Failed to fetch cameras");
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
      alert("Camera added");
      addCameraDialogOpen = false;
      refreshCameras();
    } else {
      alert("Add Camera failed");
    }
  }

  async function removeCamera(cameraId: number) {
    const response = await fetch(`/api/cameras/${cameraId}`, {
      method: "DELETE",
    });

    if (response.ok) {
      alert("Camera removed");
      refreshCameras();
    } else {
      alert("Remove Camera failed");
    }
  }

  onMount(() => {
    socket = new WebSocket(`ws://${window.location.host}/api/ws`);
    socket.addEventListener("message", onMessage);
  });

  onDestroy(() => {
    socket.removeEventListener("message", onMessage);
    socket.close();
  });
</script>

<div class="flex min-h-screen w-full flex-col">
  <header
    class="sticky top-0 flex h-16 items-center gap-4 border-b bg-background px-4 md:px-6"
  >
    <nav
      class="hidden flex-col gap-6 text-lg font-medium md:flex md:flex-row md:items-center md:gap-5 md:text-sm lg:gap-6"
    >
      <a
        href="##"
        class="flex items-center gap-2 text-lg font-semibold md:text-base"
      >
        <Package2 class="h-6 w-6" />
        <span class="sr-only">Oko</span>
      </a>
      <a
        href="##"
        class="text-foreground transition-colors hover:text-foreground"
      >
        Cameras
      </a>
      <a
        href="##"
        class="text-muted-foreground transition-colors hover:text-foreground"
      >
        Settings
      </a>
    </nav>
    <Sheet.Root>
      <Sheet.Trigger asChild let:builder>
        <Button
          variant="outline"
          size="icon"
          class="shrink-0 md:hidden"
          builders={[builder]}
        >
          <Menu class="h-5 w-5" />
          <span class="sr-only">Toggle navigation menu</span>
        </Button>
      </Sheet.Trigger>
      <Sheet.Content side="left">
        <nav class="grid gap-6 text-lg font-medium">
          <a href="##" class="flex items-center gap-2 text-lg font-semibold">
            <Package2 class="h-6 w-6" />
            <span class="sr-only">Oko</span>
          </a>
          <a href="##" class="text-muted-foreground hover:text-foreground">
            Cameras
          </a>
          <a href="##" class="hover:text-foreground"> Settings </a>
        </nav>
      </Sheet.Content>
    </Sheet.Root>
    <div class="flex w-full items-center gap-4 md:ml-auto md:gap-2 lg:gap-4">
      <div class="ml-auto flex-1 sm:flex-initial"></div>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild let:builder>
          <Button
            id="user-menu-button"
            builders={[builder]}
            variant="secondary"
            size="icon"
            class="rounded-full"
          >
            <CircleUser class="h-5 w-5" />
            <span class="sr-only">Toggle user menu</span>
          </Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content id="user-menu" align="end">
          <DropdownMenu.Label>My Account</DropdownMenu.Label>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>Settings</DropdownMenu.Item>
          <DropdownMenu.Item>Support</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item id="logout" on:click={logout}
            >Logout</DropdownMenu.Item
          >
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </div>
  </header>
  <main
    class="flex min-h-[calc(100vh_-_theme(spacing.16))] flex-1 flex-col gap-4 bg-muted/40 p-4 md:gap-8 md:p-10"
  >
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
                class={"text-muted-foreground transition-all hover:text-primary" +
                  (camera.camera_name === "Kitchen"
                    ? " font-semibold text-primary"
                    : "")}>{camera.camera_name}</a
              >
              <Button
                on:click={() => removeCamera(camera.camera_id)}
                variant="ghost"
                size="icon"
                class="ml-auto flex h-4 w-4 shrink-0 items-center justify-center opacity-0 transition-all group-hover:opacity-100"
              >
                <Trash class="h-4 w-4" />
              </Button>
            </div>
          {/each}
          {#if $user?.user?.username === "admin"}
            <Dialog.Root bind:open={addCameraDialogOpen}>
              <Dialog.Trigger
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
                    <Button type="submit">Add Camera</Button>
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
      </div>
    </div>
  </main>
</div>
