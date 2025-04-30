<script lang="ts">
  import Trash from "lucide-svelte/icons/trash";
  import CirclePlus from "lucide-svelte/icons/circle-plus";
  import Settings from "lucide-svelte/icons/settings";

  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import * as Avatar from "$lib/components/ui/avatar/index.js";
  import * as Select from "$lib/components/ui/select/index.js";

  import { user as userStore } from "../lib/stores/userStore";
  import DashboardLayout from "$lib/layouts/DashboardLayout.svelte";
  import { type User } from "../types";
  import { Plus, X } from "lucide-svelte";
  import { Separator } from "$lib/components/ui/separator";
  import { Switch } from "$lib/components/ui/switch";

  const DEFAULT_USER_NAME = "fabrice";

  let username = import.meta.env.DEV ? DEFAULT_USER_NAME : "";
  let password = "";

  let addUserDialogOpen = false;
  let getUsersPromise = getUsers();

  const refreshUsers = () => {
    getUsersPromise = getUsers();
  };

  async function getUsers(): Promise<User[]> {
    const response = await fetch("/api/users");

    if (response.ok) {
      const data = await response.json();
      return data;
    } else {
      console.error("Failed to fetch users");
      throw new Error("Failed to fetch users");
    }
  }

  async function addUser() {
    const response = await fetch("/api/users", {
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
      addUserDialogOpen = false;
      refreshUsers();
    } else {
      console.error("Add User failed");
    }
  }

  async function removeUser(userId: number) {
    const response = await fetch(`/api/users/${userId}`, {
      method: "DELETE",
    });

    if (response.ok) {
      refreshUsers();
    } else {
      console.error("Remove User failed");
    }
  }

  async function onSaveUser({ target }: Event, user: User) {
    console.dir(target);
    const formData = new FormData(target as HTMLFormElement);

    let data = {
      ...Object.fromEntries(formData.entries()),
    };

    const response = await fetch(`/api/users/${user.user_id}`, {
      method: "PATCH",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams(data as unknown as URLSearchParams), // Too lazy to manually convert everything to string
    });

    if (response.ok) {
      refreshUsers();
    } else {
      console.error("Save Users failed");
    }
  }
</script>

<DashboardLayout tab="Users">
  <div class="mx-auto grid w-full max-w-6xl gap-2">
    <h1 class="text-3xl font-semibold">Users</h1>
  </div>
  <div
    class="mx-auto grid w-full max-w-6xl items-start gap-6 md:grid-cols-[180px_1fr] lg:grid-cols-[250px_1fr]"
  >
    <nav
      class="grid gap-4 text-sm text-muted-foreground"
      data-x-chunk-container="chunk-container after:right-0"
    >
      {#await getUsersPromise}
        <!-- TODO: Use skeletons -->
        <span class="px-3 py-0 text-muted-foreground">Loading...</span>
      {:then users}
        {#each users as user}
          <div
            class="group flex items-center justify-between gap-3 rounded-lg px-3 py-0"
          >
            <Button
              data-user-id={user.user_id}
              class="h-0 px-0 py-1 font-normal text-muted-foreground transition-all hover:text-primary hover:no-underline"
              variant="link"
            >
              {user.username}
            </Button>
            <div class="flex gap-3">
              {#if user.username !== "guest"}
                <Dialog.Root>
                  <Dialog.Trigger
                    id="edit-user"
                    aria-label="Edit User"
                    data-user-id={user.user_id}
                    class={buttonVariants({ variant: "ghost", size: "icon" }) +
                      " ml-auto flex !h-4 !w-4 shrink-0 items-center justify-center transition-all group-hover:opacity-100 md:opacity-0"}
                  >
                    <Settings class="h-4 w-4" />
                  </Dialog.Trigger>
                  <Dialog.Content class="sm:max-w-[425px]">
                    <form
                      class="contents"
                      on:submit|preventDefault={(event) =>
                        onSaveUser(event, user)}
                    >
                      <Dialog.Header>
                        <Dialog.Title>Edit User</Dialog.Title>
                      </Dialog.Header>
                      <div class="grid gap-4 py-4">
                        <div class="grid grid-cols-4 items-center gap-4">
                          <Label for="username" class="text-right"
                            >Username</Label
                          >
                          <Input
                            id="username"
                            name="username"
                            placeholder="joe"
                            minlength={1}
                            max={255}
                            disabled={user.username === "admin"}
                            required
                            value={user.username}
                            class="col-span-3"
                          />
                        </div>
                        <div class="grid grid-cols-4 items-center gap-4">
                          <Label for="password" class="text-right"
                            >Password</Label
                          >
                          <Input
                            id="password"
                            name="password"
                            type="password"
                            placeholder="leave blank for no change"
                            max={255}
                            class="col-span-3"
                          />
                        </div>
                      </div>
                      <Dialog.Footer>
                        <Button variant="outline" type="submit">Save</Button>
                      </Dialog.Footer>
                    </form>
                  </Dialog.Content>
                </Dialog.Root>
              {/if}
              {#if user.username !== "admin"}
                <Button
                  on:click={() => removeUser(user.user_id)}
                  variant="ghost"
                  size="icon"
                  aria-label="Remove User"
                  data-user-id={user.user_id}
                  class="ml-auto flex h-4 w-4 shrink-0 items-center justify-center transition-all group-hover:opacity-100 md:opacity-0"
                >
                  <Trash class="h-4 w-4" />
                </Button>
              {/if}
            </div>
          </div>
        {/each}
        <Dialog.Root bind:open={addUserDialogOpen}>
          <Dialog.Trigger
            id="add-user"
            class={buttonVariants({ variant: "outline" }) + " gap-1"}
          >
            <CirclePlus class="h-3.5 w-3.5" />
            Add User
          </Dialog.Trigger>
          <Dialog.Content class="sm:max-w-[425px]">
            <form class="contents" on:submit|preventDefault={addUser}>
              <Dialog.Header>
                <Dialog.Title>Add User</Dialog.Title>
              </Dialog.Header>
              <div class="grid gap-4 py-4">
                <div class="grid grid-cols-4 items-center gap-4">
                  <Label for="username" class="text-right">Username</Label>
                  <Input
                    id="username"
                    placeholder="joe"
                    minlength={1}
                    max={255}
                    required
                    bind:value={username}
                    class="col-span-3"
                  />
                </div>
                <div class="grid grid-cols-4 items-center gap-4">
                  <Label for="password" class="text-right">Password</Label>
                  <Input
                    id="password"
                    type="password"
                    placeholder="12345"
                    minlength={1}
                    max={255}
                    required
                    disabled={username === "guest"}
                    bind:value={password}
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
      {:catch error}
        <p>{error.message}</p>
      {/await}
    </nav>
  </div>
</DashboardLayout>
