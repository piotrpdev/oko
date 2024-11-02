<script lang="ts">
  import PWABadge from './lib/PWABadge.svelte'

  async function handleSubmit(event: Event) {
    const response = await fetch('/api/login', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded'
      },
      body: new URLSearchParams({
        username: (event.target as HTMLFormElement).username.value,
        password: (event.target as HTMLFormElement).password.value
      })
    })

    if (response.ok) {
      alert('Login successful')
    } else {
      alert('Login failed')
    }
  }

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
      alert('Logout successful')
    } else {
      alert('Logout failed')
    }
  }
</script>

<main>
  <form on:submit|preventDefault={handleSubmit}>
    <fieldset>
      <legend>User login</legend>
      <p>
        <label for="username">Username</label>
        <input name="username" id="username" value="admin" />
      </p>
      <p>
        <label for="password">Password</label>
        <input
          name="password"
          id="password"
          type="password"
          value="hunter42"
        />
      </p>
    </fieldset>

    <button type="submit">Login</button>
  </form>

  <div>
    <button on:click={getData}>Get Data</button>
    <button on:click={logout}>Logout</button>
  </div>
</main>

<PWABadge />
