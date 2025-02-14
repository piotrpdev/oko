<script lang="ts">
  export let tab = "";

  import CircleUser from "lucide-svelte/icons/circle-user";
  import Menu from "lucide-svelte/icons/menu";
  import Package2 from "lucide-svelte/icons/package-2";
  import Eye from "lucide-svelte/icons/eye";

  import { Button } from "$lib/components/ui/button/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Sheet from "$lib/components/ui/sheet/index.js";

  import { replace, link } from "svelte-spa-router";
  import { user } from "../stores/userStore";

  let tabs = [
    { name: "Home", href: "/" },
    { name: "Cameras", href: "/cameras" },
  ];

  async function logout() {
    const response = await fetch("/api/logout");

    if (response.ok) {
      $user = null;
      replace("/login");
    } else {
      console.error("Logout failed");
    }
  }
</script>

<div class="flex min-h-screen w-full flex-col">
  <header
    class="sticky top-0 z-10 flex h-16 items-center gap-4 border-b bg-background px-4 md:px-6"
  >
    <nav
      class="hidden flex-col gap-6 text-lg font-medium md:flex md:flex-row md:items-center md:gap-5 md:text-sm lg:gap-6"
    >
      <span class="flex items-center gap-2 text-lg font-semibold md:text-base">
        <Eye class="h-6 w-6" />
        <span class="sr-only">Oko</span>
      </span>
      {#each tabs as { name, href }}
        <a
          {href}
          class={`${tab === name ? "text-foreground" : "text-muted-foreground"} transition-colors hover:text-foreground`}
          use:link
        >
          {name}
        </a>
      {/each}
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
          <span class="flex items-center gap-2 text-lg font-semibold">
            <Package2 class="h-6 w-6" />
            <span class="sr-only">Oko</span>
          </span>
          {#each tabs as { name, href }}
            <a
              {href}
              class={`${tab === name ? "text-foreground" : "text-muted-foreground"} transition-colors hover:text-foreground`}
              use:link
            >
              {name}
            </a>
          {/each}
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
    <slot />
  </main>
</div>
