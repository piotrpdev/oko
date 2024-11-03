<script lang="ts">
  import { replace } from "svelte-spa-router";

  async function getData() {
    const response = await fetch('/api/')

    if (response.redirected) {
      alert('You need to login first')
      return
    }

    if (response.ok) {
      const data = await response.json()
      alert(JSON.stringify(data))
    } else {
      alert('Failed to get data')
    }
  }

  async function logout() {
    const response = await fetch('/api/logout')

    if (response.ok) {
      replace('/login')
    } else {
      alert('Logout failed')
    }
  }
</script>

<main>
  <div>
    <button on:click={getData}>Get Data</button>
    <button on:click={logout}>Logout</button>
  </div>
</main>
