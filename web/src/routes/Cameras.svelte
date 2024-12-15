<script lang="ts">
  import Trash from "lucide-svelte/icons/trash";
  import CirclePlus from "lucide-svelte/icons/circle-plus";
  import Settings from "lucide-svelte/icons/settings";

  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";

  import ChevronDown from "svelte-radix/ChevronDown.svelte";
  import * as Avatar from "$lib/components/ui/avatar/index.js";
  import * as Command from "$lib/components/ui/command/index.js";
  import * as Popover from "$lib/components/ui/popover/index.js";
  import * as Select from "$lib/components/ui/select/index.js";

  import { user } from "../lib/stores/userStore";
  import DashboardLayout from "$lib/layouts/DashboardLayout.svelte";
  import { type Camera, type CameraPermission } from "../types";
  import CameraAndVideos from "$lib/components/CameraAndVideos.svelte";
  import { Check } from "lucide-svelte";
  import { cn } from "$lib/utils";
  import type { Selected } from "bits-ui";

  let selectedCameraId: number | null = null;
  let selectedCameraName: string | null = null;

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
      refreshCameras();
    } else {
      console.error("Remove Camera failed");
    }
  }

  let getPermissionsPromise: Promise<CameraPermission[]> = Promise.resolve([]);
  const refreshPermissions = (cameraId: number) =>
    (getPermissionsPromise = getPermissions(cameraId));

  function editCameraDialogOpenChange(isOpen: boolean, cameraId: number) {
    if (!isOpen) return;

    getPermissionsPromise = getPermissions(cameraId);
  }

  const userRoleOptions = [
    {
      value: "none",
      label: "None",
    },
    {
      value: "viewer",
      label: "Viewer",
    },
    {
      value: "controller",
      label: "Controller",
    },
  ];

  async function getPermissions(cameraId: number): Promise<CameraPermission[]> {
    const response = await fetch(`/api/cameras/${cameraId}/permissions`);

    if (response.ok) {
      const data = await response.json();
      return data;
    } else {
      console.error(`Failed to fetch permissions for camera ${cameraId}`);
      throw new Error(`Failed to fetch permissions for camera ${cameraId}`);
    }
  }

  function permissionToRole(permission: CameraPermission): string {
    if (permission.can_control) {
      return "Controller";
    } else if (permission.can_view) {
      return "Viewer";
    } else {
      return "None";
    }
  }

  function roleToPermission(role: string): {
    can_view: string;
    can_control: string;
  } {
    switch (role) {
      case "viewer":
        return { can_view: "true", can_control: "false" };
      case "controller":
        return { can_view: "true", can_control: "true" };
      default:
        return { can_view: "false", can_control: "false" };
    }
  }

  async function onPermissionChange(
    permission: CameraPermission,
    selected?: Selected<string>,
  ) {
    if (!selected) return;

    const response = await fetch(
      `/api/permissions/${permission.permission_id}`,
      {
        method: "PATCH",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: new URLSearchParams({
          ...roleToPermission(selected.value),
        }),
      },
    );

    if (response.ok) {
      // refreshPermissions(permission.camera_id);
    } else {
      console.error("Update Permission failed");
    }

    refreshPermissions(permission.camera_id);
  }
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
          <div
            class="group flex items-center justify-between gap-3 rounded-lg px-3 py-0"
          >
            <Button
              on:click={() => {
                selectedCameraId = camera.camera_id;
                selectedCameraName = camera.camera_name;
              }}
              data-camera-id={camera.camera_id}
              aria-label="View Camera"
              class={"h-0 px-0 py-1 font-normal text-muted-foreground transition-all hover:text-primary hover:no-underline" +
                (camera.camera_id === selectedCameraId
                  ? " font-semibold text-primary"
                  : "")}
              variant="link"
            >
              {camera.camera_name}
            </Button>
            <div class="flex gap-3">
              <!-- TODO: Check if user has permissions to control instead of being admin -->
              {#if $user?.user?.username === "admin"}
                <Dialog.Root
                  onOpenChange={(isOpen) =>
                    editCameraDialogOpenChange(isOpen, camera.camera_id)}
                >
                  <Dialog.Trigger
                    aria-label="Edit Camera"
                    data-camera-id={camera.camera_id}
                    class={buttonVariants({ variant: "ghost", size: "icon" }) +
                      " ml-auto flex !h-4 !w-4 shrink-0 items-center justify-center transition-all group-hover:opacity-100 md:opacity-0"}
                  >
                    <Settings class="h-4 w-4" />
                  </Dialog.Trigger>
                  <!-- TODO: Fix layout shift caused by await -->
                  <Dialog.Content class="sm:max-w-[425px]">
                    <form class="contents">
                      <Dialog.Header>
                        <Dialog.Title>Edit Camera</Dialog.Title>
                      </Dialog.Header>
                      <div class="grid gap-4 py-4">
                        {#await getPermissionsPromise}
                          <!-- TODO: Use skeletons -->
                          <span class="px-3 py-0 text-muted-foreground"
                            >Loading...</span
                          >
                        {:then permissions}
                          {#each permissions.filter((p) => p.username !== "admin") as permission}
                            <div class="flex items-center justify-between">
                              <div class="flex items-center space-x-4">
                                <Avatar.Root>
                                  <Avatar.Image alt={permission.username} />
                                  <Avatar.Fallback
                                    >{permission.username
                                      .substring(0, 2)
                                      .toUpperCase()}</Avatar.Fallback
                                  >
                                </Avatar.Root>
                                <div>
                                  <p class="text-sm font-medium leading-none">
                                    {permission.username}
                                  </p>
                                </div>
                              </div>
                              <Select.Root
                                selected={userRoleOptions.find(
                                  (option) =>
                                    option.label ===
                                    permissionToRole(permission),
                                )}
                                onSelectedChange={(selected) =>
                                  onPermissionChange(permission, selected)}
                              >
                                <Select.Trigger class="w-[120px]">
                                  <Select.Value placeholder="Role" />
                                </Select.Trigger>
                                <Select.Content>
                                  <Select.Group>
                                    {#each userRoleOptions as userRole}
                                      <Select.Item
                                        value={userRole.value}
                                        label={userRole.label}
                                        >{userRole.label}</Select.Item
                                      >
                                    {/each}
                                  </Select.Group>
                                </Select.Content>
                              </Select.Root>
                            </div>
                          {/each}
                        {:catch error}
                          <p>{error.message}</p>
                        {/await}
                      </div>
                    </form>
                  </Dialog.Content>
                </Dialog.Root>
              {/if}
              <Button
                on:click={() => removeCamera(camera.camera_id)}
                variant="ghost"
                size="icon"
                aria-label="Remove Camera"
                data-camera-id={camera.camera_id}
                class="ml-auto flex h-4 w-4 shrink-0 items-center justify-center transition-all group-hover:opacity-100 md:opacity-0"
              >
                <Trash class="h-4 w-4" />
              </Button>
            </div>
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
      {#if selectedCameraId !== null && selectedCameraName !== null}
        <CameraAndVideos
          cameraId={selectedCameraId}
          cameraName={selectedCameraName}
        />
      {/if}
    </div>
  </div>
</DashboardLayout>
