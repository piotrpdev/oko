<script lang="ts">
  import { useRegisterSW } from "virtual:pwa-register/svelte";

  // check for updates every hour
  const period = 60 * 60 * 1000;

  /**
   * This function will register a periodic sync check every hour, you can modify the interval as needed.
   */
  function registerPeriodicSync(swUrl: string, r: ServiceWorkerRegistration) {
    if (period <= 0) return;

    setInterval(async () => {
      if ("onLine" in navigator && !navigator.onLine) return;

      const resp = await fetch(swUrl, {
        cache: "no-store",
        headers: {
          cache: "no-store",
          "cache-control": "no-cache",
        },
      });

      if (resp?.status === 200) await r.update();
    }, period);
  }

  const { needRefresh, updateServiceWorker } = useRegisterSW({
    onRegisteredSW(swUrl, r) {
      if (period <= 0) return;
      if (r?.active?.state === "activated") {
        registerPeriodicSync(swUrl, r);
      } else if (r?.installing) {
        r.installing.addEventListener("statechange", (e) => {
          const sw = e.target as ServiceWorker;
          if (sw.state === "activated") registerPeriodicSync(swUrl, r);
        });
      }
    },
  });

  function close() {
    needRefresh.set(false);
  }

  $: toast = $needRefresh;
  $: message = $needRefresh
    ? "New content available, click on reload button to update."
    : "";
</script>

{#if toast}
  <div
    class="pwa-toast bg-background text-sm"
    role="alert"
    aria-labelledby="toast-message"
  >
    <div class="message">
      <span id="toast-message">
        {message}
      </span>
    </div>
    <div class="buttons font-medium">
      {#if $needRefresh}
        <button
          class="bg-primary text-primary-foreground"
          type="button"
          on:click={() => updateServiceWorker(true)}
        >
          Reload
        </button>
      {/if}
      <button
        class="bg-primary-foreground text-primary"
        type="button"
        on:click={close}
      >
        Close
      </button>
    </div>
  </div>
{/if}

<style>
  /* TODO: Change color scheme to dark */
  .pwa-toast {
    position: fixed;
    right: 0;
    bottom: 0;
    margin: 16px;
    padding: 12px;
    border: 1px solid #8885;
    border-radius: 4px;
    z-index: 2;
    text-align: left;
    /* box-shadow: 3px 4px 5px 0 #8885; */
  }
  .pwa-toast .message {
    margin-bottom: 12px;
  }
  .pwa-toast .buttons {
    display: flex;
  }
  .pwa-toast button {
    border: 1px solid #8885;
    outline: none;
    margin-right: 5px;
    border-radius: 2px;
    padding: 3px 10px;
  }
</style>
