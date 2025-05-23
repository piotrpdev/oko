<script lang="ts">
  import Trash from "lucide-svelte/icons/trash";
  import CirclePlus from "lucide-svelte/icons/circle-plus";
  import Settings from "lucide-svelte/icons/settings";

  import * as Table from "$lib/components/ui/table/index.js";
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
  import {
    isCameraListChange,
    isImageContainer,
    isMdnsCamera,
    type Camera,
    type CameraPermission,
    type CameraSetting,
    type ImageContainer,
    type MdnsCamera,
  } from "../types";
  import CameraAndVideos from "$lib/components/CameraAndVideos.svelte";
  import { Check, Plus, X } from "lucide-svelte";
  import { cn } from "$lib/utils";
  import type { Selected } from "bits-ui";
  import { Separator } from "$lib/components/ui/separator";
  import { Switch } from "$lib/components/ui/switch";
  import { onDestroy, onMount } from "svelte";
  import { socket } from "$lib/stores/socketStore";

  const DEFAULT_CAMERA_NAME = "Backyard";
  const DEFAULT_CAMERA_ADDRESS = "192.168.0.30";

  let selectedCameraId: number | null = null;
  let selectedCameraName: string | null = null;

  let name = import.meta.env.DEV ? DEFAULT_CAMERA_NAME : "";
  let address = import.meta.env.DEV ? DEFAULT_CAMERA_ADDRESS : "";

  // TODO: Refresh cameras on add/remove
  // TODO: Better indication that "Save Settings" button isn't needed for "User Permissions"
  // TODO: Display toast on API errors?
  // ? Maybe use a store for cameras
  // ? Maybe show confirmation dialog on remove

  let addCameraDialogOpen = false;
  let getCamerasPromise = getCameras();

  const refreshCameras = () => {
    getCamerasPromise = getCameras();
    getCamerasPromise.then((cameras) => {
      const camera = cameras.find(
        (camera) => camera.camera_id === selectedCameraId,
      );

      if (!camera) {
        selectedCameraId = null;
        selectedCameraName = null;
        return;
      }

      selectedCameraName = camera.camera_name;
    });
  };

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

  let cameraResolutionOptions = [
    {
      value: "SVGA",
      label: "SVGA",
    },
    {
      value: "VGA",
      label: "VGA",
    },
  ];

  let getSettingsPromise: Promise<CameraSetting> = Promise.resolve({
    camera_id: -1,
    flashlight_enabled: false,
    framerate: -1,
    last_modified: [],
    modified_by: -1,
    resolution: "",
    setting_id: -1,
  });
  const refreshSettings = (cameraId: number) =>
    (getSettingsPromise = getSettings(cameraId));

  async function getSettings(cameraId: number): Promise<CameraSetting> {
    const response = await fetch(`/api/cameras/${cameraId}/settings`);

    if (response.ok) {
      const data = await response.json();
      return data;
    } else {
      console.error(`Failed to fetch settings for camera ${cameraId}`);
      throw new Error(`Failed to fetch settings for camera ${cameraId}`);
    }
  }

  function editCameraDialogOpenChange(isOpen: boolean, cameraId: number) {
    if (!isOpen) return;

    getPermissionsPromise = getPermissions(cameraId);
    getSettingsPromise = getSettings(cameraId);
  }

  let selectedResolution: string | undefined;

  async function onSaveSettings({ target }: Event, setting: CameraSetting) {
    const formData = new FormData(target as HTMLFormElement);

    // Shadcn switch uses undefined for unchecked
    let data = {
      ...setting,
      flashlight_enabled: "false",
      ...(selectedResolution !== undefined
        ? { resolution: selectedResolution }
        : {}),
      ...Object.fromEntries(formData.entries()),
    };

    const response = await fetch(`/api/settings/${setting.setting_id}`, {
      method: "PATCH",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams(data as unknown as URLSearchParams), // Too lazy to manually convert everything to string
    });

    if (response.ok) {
      refreshSettings(setting.setting_id);
    } else {
      console.error("Save Settings failed");
    }
  }

  async function handleRestartCamera(cameraId: number) {
    const response = await fetch(`/api/cameras/${cameraId}/restart`, {
      method: "POST",
    });

    if (response.ok) {
      refreshSettings(cameraId);
    } else {
      console.error("Restart Camera failed");
    }
  }

  let mdnsCameras: MdnsCamera[] = [];
  let mdnsCamerasSse: EventSource | null = null;

  function onMdnsCamerasSseMessage({ data }: MessageEvent) {
    try {
      const parsed_msg = JSON.parse(data);

      if (!isMdnsCamera(parsed_msg)) {
        console.error("Parsed invalid mDNS camera message");
        return;
      }

      if (
        mdnsCameras.find(
          (camera) => camera.socket_address === parsed_msg.socket_address,
        )
      ) {
        return;
      }

      mdnsCameras = [...mdnsCameras, parsed_msg];
    } catch (e) {
      console.error("Failed to parse mDNS camera message JSON");
    }
  }

  function closeMdnsCamerasSse() {
    mdnsCamerasSse?.removeEventListener("message", onMdnsCamerasSseMessage);
    mdnsCamerasSse?.close();
    mdnsCamerasSse = null;
  }

  // TODO: move contents to editCameraDialogOpenChange ?
  $: (async (_addCameraDialogOpen) => {
    // Need this for tests to run correctly
    if (typeof EventSource === "undefined") {
      console.error("EventSource is not supported in this browser");
      return;
    }

    if (!_addCameraDialogOpen) {
      closeMdnsCamerasSse();
      return;
    }

    if (mdnsCamerasSse != null) return;

    mdnsCameras = [];
    mdnsCamerasSse = new EventSource("/api/mdns_cameras_sse");
    mdnsCamerasSse?.addEventListener("message", onMdnsCamerasSseMessage);
  })(addCameraDialogOpen);

  let processImage:
    | ((image_bytes: ImageContainer["image_bytes"]) => void)
    | undefined;

  function onMessage(event: MessageEvent) {
    const data = event.data;

    try {
      const parsed_msg = JSON.parse(data);

      if (isImageContainer(parsed_msg)) {
        if (parsed_msg.camera_id !== selectedCameraId) {
          return;
        }

        processImage?.(parsed_msg.image_bytes);
      } else if (isCameraListChange(parsed_msg)) {
        console.log("Camera list changed");

        if ($user?.user?.username === "admin") {
          return;
        }

        refreshCameras();
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
    closeMdnsCamerasSse();
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
              {#if camera.can_control}
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
                    <div class="contents">
                      <Dialog.Header>
                        <Dialog.Title>Edit Camera</Dialog.Title>
                      </Dialog.Header>
                      {#if $user?.user?.username === "admin"}
                        <div class="grid gap-4 pt-4">
                          <h4 class="text-sm font-medium">User Permissions</h4>
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
                                  <Select.Trigger
                                    aria-label="Edit User Camera Permission"
                                    data-permission-id={permission.permission_id}
                                    class="w-[120px]"
                                  >
                                    <Select.Value
                                      aria-label="Current User Camera Permission"
                                      data-permission-id={permission.permission_id}
                                      placeholder="Role"
                                    />
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
                        <Separator class="my-2" />
                      {/if}
                      <div class="grid gap-4">
                        <h4 class="text-sm font-medium">Settings</h4>
                        {#await getSettingsPromise}
                          <!-- TODO: Use skeletons -->
                          <span class="px-3 py-0 text-muted-foreground"
                            >Loading...</span
                          >
                        {:then settings}
                          <form
                            class="contents"
                            on:submit|preventDefault={(event) =>
                              onSaveSettings(event, settings)}
                          >
                            <div
                              class="flex items-center justify-between space-x-2"
                            >
                              <Label for="flashlight" class="flex flex-col">
                                <span class="font-normal">Flashlight</span>
                              </Label>
                              <Switch
                                id="flashlight"
                                aria-label="Flashlight"
                                name="flashlight_enabled"
                                value="true"
                                checked={settings.flashlight_enabled}
                              />
                            </div>
                            {#if $user?.user?.username === "admin"}
                              <div
                                class="flex items-center justify-between space-x-2"
                              >
                                <Label for="framerate" class="flex flex-col">
                                  <span class="font-normal"
                                    >Framerate (FPS)</span
                                  >
                                  <span
                                    class="text-xs font-normal leading-snug text-muted-foreground"
                                  >
                                    requires camera restart
                                  </span>
                                </Label>
                                <Input
                                  class="w-[4.5rem]"
                                  id="framerate"
                                  type="number"
                                  name="framerate"
                                  min={1}
                                  max={60}
                                  required
                                  placeholder="5"
                                  value={settings.framerate}
                                />
                              </div>
                              <div
                                class="flex items-center justify-between space-x-2"
                              >
                                <Label for="framerate" class="flex flex-col">
                                  <span class="font-normal">Resolution</span>
                                  <span
                                    class="text-xs font-normal leading-snug text-muted-foreground"
                                  >
                                    requires camera restart
                                  </span>
                                </Label>
                                <Select.Root
                                  selected={cameraResolutionOptions.find(
                                    (option) =>
                                      option.label === settings.resolution,
                                  )}
                                  onSelectedChange={(selected) =>
                                    selected &&
                                    (selectedResolution = selected.value)}
                                >
                                  <Select.Trigger
                                    aria-label="Edit Camera Resolution"
                                    data-permission-id={settings.setting_id}
                                    class="w-[120px]"
                                  >
                                    <Select.Value
                                      aria-label="Current Camera Resolution"
                                      data-permission-id={settings.setting_id}
                                      placeholder="Resolution"
                                    />
                                  </Select.Trigger>
                                  <Select.Content>
                                    <Select.Group>
                                      {#each cameraResolutionOptions as cameraResolution}
                                        <Select.Item
                                          value={cameraResolution.value}
                                          label={cameraResolution.label}
                                          >{cameraResolution.label}</Select.Item
                                        >
                                      {/each}
                                    </Select.Group>
                                  </Select.Content>
                                </Select.Root>
                              </div>
                            {/if}
                            <Separator class="h-0" />
                            <Button
                              id="save-settings"
                              variant="outline"
                              class="w-full"
                              type="submit">Save Settings</Button
                            >
                          </form>
                          {#if $user?.user?.username === "admin"}
                            <Button
                              id="restart-camera"
                              variant="destructive"
                              class="w-full"
                              type="button"
                              on:click={() =>
                                handleRestartCamera(camera.camera_id)}
                            >
                              Restart Camera
                            </Button>
                          {/if}
                        {:catch error}
                          <p>{error.message}</p>
                        {/await}
                      </div>
                    </div>
                  </Dialog.Content>
                </Dialog.Root>
              {/if}
              {#if $user?.user?.username === "admin"}
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
              {/if}
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
                  <Separator class="my-2" />
                  <h4 class="text-sm font-medium">mDNS Cameras</h4>
                  {#await getCamerasPromise}
                    <!-- TODO: Use skeletons -->
                    <span class="px-3 py-0 text-muted-foreground"
                      >Loading...</span
                    >
                  {:then cameras}
                    <!-- TODO: Sort by creation date -->
                    {#each mdnsCameras as mdnsCamera}
                      <div class="flex items-center justify-between gap-x-2">
                        <Label
                          for={`mdns-camera-${mdnsCamera.socket_address}`}
                          class="flex flex-col gap-1"
                        >
                          <span class="font-normal">{mdnsCamera.hostname}</span>
                          <span
                            class="text-xs font-normal leading-snug text-muted-foreground"
                          >
                            {mdnsCamera.socket_address.split(":")[0]}
                            {#if cameras.some((camera) => camera.ip_address.split(":")[0] === mdnsCamera.socket_address.split(":")[0])}
                              (taken by "{`${cameras.find((camera) => camera.ip_address.split(":")[0] === mdnsCamera.socket_address.split(":")[0])?.camera_name}`}")
                            {/if}
                          </span>
                        </Label>
                        <Button
                          id={`mdns-camera-${mdnsCamera.socket_address}`}
                          variant="outline"
                          size="icon"
                          aria-label="Select"
                          data-camera-ip={mdnsCamera.socket_address}
                          on:click={() => {
                            name = mdnsCamera.hostname.includes("oko_camera")
                              ? (mdnsCamera.hostname
                                  .split(".")[0]
                                  .split("_")
                                  .at(-1) ?? "mDNS Camera")
                              : "mDNS Camera";
                            address = mdnsCamera.socket_address.split(":")[0];
                          }}
                          disabled={cameras.some(
                            (camera) =>
                              camera.ip_address.split(":")[0] ===
                              mdnsCamera.socket_address.split(":")[0],
                          )}
                        >
                          {#if cameras.some((camera) => camera.ip_address.split(":")[0] === mdnsCamera.socket_address.split(":")[0])}
                            <X class="h-4 w-4" />
                          {:else}
                            <Plus class="h-4 w-4" />
                          {/if}
                        </Button>
                      </div>
                    {/each}
                  {:catch error}
                    <p>{error.message}</p>
                  {/await}
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
          bind:processImage
        />
      {/if}
    </div>
  </div>
</DashboardLayout>
