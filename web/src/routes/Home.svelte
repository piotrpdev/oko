<script lang="ts">
  import CircleUser from "lucide-svelte/icons/circle-user";
  import Menu from "lucide-svelte/icons/menu";
  import Package2 from "lucide-svelte/icons/package-2";

  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Sheet from "$lib/components/ui/sheet/index.js";

  import { replace } from "svelte-spa-router";
  import { user } from "../lib/userStore";
  import { onDestroy, onMount } from "svelte";

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
        <a href="##" class="font-semibold text-primary"> Kitchen </a>
        <a href="##">Front Door</a>
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
            <img
              id="live-feed"
              src={imgSrc}
              width="800px"
              height="600px"
              alt="live feed"
              style={`visibility: ${imgSrc ? "visible" : "hidden"}`}
            />
          </Card.Content>
        </Card.Root>
      </div>
    </div>
  </main>
</div>
