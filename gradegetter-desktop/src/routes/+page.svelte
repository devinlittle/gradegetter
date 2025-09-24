<script>
  import { redirect } from "@sveltejs/kit";
  import { onMount, onDestroy } from "svelte";
  import { fetch } from "@tauri-apps/plugin-http";

  let LoggedIn = $state(false);
  let grades = $state({});
  let token = localStorage.getItem("token");
  let apiUrl = "api.devinlittle.net";

  let load = async () => {
    if (localStorage.getItem("token").length > 0) {
      LoggedIn = true;
    } else {
      LoggedIn = false;
    }
  };

  let fetchGrades = async () => {
    const response = await fetch(`https://${apiUrl}:3000/grades`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      console.error("Failed to fetch grades");
      //     throw new Error(`Schoology registration failed: ${msg}`);
    }

    const newGrades = await response.json();

    for (const subject in newGrades) {
      grades[subject] = newGrades[subject];
    }

    for (const subject in grades) {
      if (!(subject in newGrades)) {
        delete grades[subject];
      }
    }
  };

  onMount(() => {
    load();
    fetchGrades();

    const interval = setInterval(() => {
      fetchGrades();
    }, 5000); // every 5 seconds

    onDestroy(() => {
      clearInterval(interval);
    });
  });

  async function logOut(event) {
    event.preventDefault();
    localStorage.removeItem("token");
    LoggedIn = false;
  }
</script>

{#if LoggedIn}
  <h1><button onclick={logOut}>LogOut</button></h1>

  {#if Object.keys(grades).length === 0}
    <p>Loading...</p>
  {:else}
    {#each Object.entries(grades) as [subject, scores]}
      <h2>{subject}</h2>
      <ul>
        {#each scores as score, i}
          <li>Q{i + 1}: {score !== null ? score.toFixed(2) : "N/A"}</li>
        {/each}
      </ul>
    {/each}
  {/if}
{:else}
  <p>Logged Out...</p>

  <main class="container">
    <h1><a href="/register">Register</a></h1>
    <h1><a href="/login">Login</a></h1>
  </main>
{/if}

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte-kit:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }

  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;

    color: #0f0f0f;
    background-color: #f6f6f6;

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }

  .container {
    margin: 0;
    padding-top: 10vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
  }

  .logo {
    height: 6em;
    padding: 1.5em;
    will-change: filter;
    transition: 0.75s;
  }

  .logo.tauri:hover {
    filter: drop-shadow(0 0 2em #24c8db);
  }

  .row {
    display: flex;
    justify-content: center;
  }

  a {
    font-weight: 500;
    color: #646cff;
    text-decoration: inherit;
  }

  a:hover {
    color: #535bf2;
  }

  h1 {
    text-align: center;
  }

  input,
  button {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    color: #0f0f0f;
    background-color: #ffffff;
    transition: border-color 0.25s;
    box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
  }

  button {
    cursor: pointer;
  }

  button:hover {
    border-color: #396cd8;
  }
  button:active {
    border-color: #396cd8;
    background-color: #e8e8e8;
  }

  input,
  button {
    outline: none;
  }

  #greet-input {
    margin-right: 5px;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }

    a:hover {
      color: #24c8db;
    }

    input,
    button {
      color: #ffffff;
      background-color: #0f0f0f98;
    }
    button:active {
      background-color: #0f0f0f69;
    }
  }
</style>
