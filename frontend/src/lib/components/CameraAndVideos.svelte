<script lang="ts">
  import Download from "lucide-svelte/icons/download";
  import RotateCw from "lucide-svelte/icons/rotate-cw";

  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";

  import { onDestroy, onMount } from "svelte";
  import * as Table from "$lib/components/ui/table/index.js";
  import { socket } from "$lib/stores/socketStore";
  import { isImageContainer, type VideoCameraView } from "../../types";

  export let cameraId: number;
  export let cameraName: string;

  let frameCount = 0;
  let imgSrc: string = "";

  // Needed to reset the frame count and image source when the camera changes
  $: ((_cameraId) => {
    frameCount = 0;
    imgSrc = "";
  })(cameraId);

  $: videosPromise = getVideos(cameraId);

  const refreshVideos = () => (videosPromise = getVideos(cameraId));

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
        if (parsed_msg.camera_id !== cameraId) {
          return;
        }

        frameCount++;
        const bytes = new Uint8Array(parsed_msg.image_bytes);
        const blob = new Blob([bytes], { type: "image/jpeg" });
        const url = URL.createObjectURL(blob);
        if (imgSrc !== "") {
          URL.revokeObjectURL(imgSrc);
        }
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

<Card.Root>
  <Card.Header>
    <Card.Title>{cameraName}</Card.Title>
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
          <!-- TODO: Sort by creation date -->
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
          <Table.Row>
            <Table.Cell class="text-muted-foreground">
              {error.message}
            </Table.Cell>
          </Table.Row>
        {/await}
      </Table.Body>
    </Table.Root>
  </Card.Content>
</Card.Root>
