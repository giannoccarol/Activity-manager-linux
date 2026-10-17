<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "../lib/api";
  import { iconUrl } from "../lib/icons.svelte";
  import Icon from "../components/Icon.svelte";

  // Schermata Ctrl+Alt+Canc, come in Windows: blocca, cambia utente, esci, Gestione attività.
  const primary = (window as unknown as { __PRIMARY__?: boolean }).__PRIMARY__ ?? true;
  let info = $state<{ user: string; fullName: string; avatar: string | null; host: string } | null>(null);
  let now = $state(new Date());
  let powerOpen = $state(false);
  let list: HTMLDivElement | undefined = $state();

  const OPTIONS: [string, string, string][] = [
    ["lock", "Blocca", "lock"],
    ["switch", "Cambia utente", "users"],
    ["logout", "Esci", "logout"],
    ["taskmgr", "Gestione attività", "performance"],
  ];

  async function session(verb: string) {
    powerOpen = false;
    if (!isTauri) return console.info("[simulazione] sessione", verb);
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("session", { verb });
  }

  onMount(() => {
    if (isTauri)
      import("@tauri-apps/api/core").then(({ invoke }) => invoke<typeof info>("session_info").then((v) => (info = v)));
    else info = { user: "mario", fullName: "Mario Rossi", avatar: null, host: "workstation" };
    const t = setInterval(() => (now = new Date()), 1000);
    queueMicrotask(() => (list?.querySelector("button") as HTMLElement | null)?.focus());
    return () => clearInterval(t);
  });

  const time = $derived(now.toLocaleTimeString("it-IT", { hour: "2-digit", minute: "2-digit" }));
  const date = $derived.by(() => {
    const d = now.toLocaleDateString("it-IT", { weekday: "long", day: "numeric", month: "long" });
    return d.charAt(0).toUpperCase() + d.slice(1);
  });
  const initials = $derived(
    (info?.fullName ?? "")
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join(""),
  );
  const avatar = $derived(info?.avatar ? iconUrl(info.avatar) : null);

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (powerOpen) powerOpen = false;
      else session("cancel");
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const items = [...(list?.querySelectorAll("button") ?? [])] as HTMLElement[];
      const i = items.indexOf(document.activeElement as HTMLElement);
      const n = (i + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
      items[n]?.focus();
      e.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={key} />
<div class="screen">
  <div class="bg"></div>
  {#if primary}
    <header>
      <div class="time">{time}</div>
      <div class="date">{date}</div>
    </header>

    <main>
      <div class="who">
        <span class="avatar">
          {#if avatar}<img src={avatar} alt="" />{:else}{initials}{/if}
        </span>
        <span class="name">{info?.fullName ?? ""}</span>
        {#if info}<span class="host">{info.user} su {info.host}</span>{/if}
      </div>
      <div class="opts" bind:this={list}>
        {#each OPTIONS as [verb, label, icon], i}
          <button onclick={() => session(verb)} style:--i={i}>
            <Icon name={icon} size={18} />
            <span>{label}</span>
          </button>
        {/each}
      </div>
      <button class="cancel" onclick={() => session("cancel")}>Annulla</button>
    </main>

    <footer>
      <div class="power-wrap">
        {#if powerOpen}
          <div class="power-menu" role="menu">
            <button role="menuitem" onclick={() => session("suspend")}><Icon name="pause" size={16} />Sospendi</button>
            <button role="menuitem" onclick={() => session("reboot")}><Icon name="restart" size={16} />Riavvia…</button>
            <button role="menuitem" onclick={() => session("shutdown")}><Icon name="power" size={16} />Arresta il sistema…</button>
          </div>
        {/if}
        <button class="power" aria-label="Alimentazione" aria-expanded={powerOpen} onclick={() => (powerOpen = !powerOpen)}>
          <Icon name="power" size={20} stroke={1.8} />
        </button>
      </div>
    </footer>
  {/if}
</div>

<style>
  .screen {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    color: #fff;
    overflow: hidden;
    animation: fade 220ms var(--ease);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  /* la finestra è trasparente: KWin sfoca il desktop dietro, qui solo la velatura */
  .bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse at center, rgb(8 13 20 / 0.15), rgb(8 13 20 / 0.45) 78%),
      rgb(12 17 24 / 0.62);
  }
  header,
  main,
  footer {
    position: relative;
  }
  header {
    margin-top: 9vh;
    text-align: center;
  }
  .time {
    font-size: 104px;
    font-weight: 400;
    font-stretch: 88%;
    letter-spacing: -0.02em;
    line-height: 1;
  }
  .date {
    margin-top: 10px;
    font-size: 21px;
    color: rgb(255 255 255 / 0.82);
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
    margin-bottom: 9vh;
    animation: rise 320ms var(--ease);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  .who {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .avatar {
    animation: avatar-in 420ms var(--ease-spring) 60ms both;
    display: grid;
    place-items: center;
    width: 88px;
    height: 88px;
    margin-bottom: 8px;
    overflow: hidden;
    border-radius: 50%;
    font-size: 30px;
    font-weight: 500;
    background: rgb(255 255 255 / 0.14);
    border: 1px solid rgb(255 255 255 / 0.28);
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  @keyframes avatar-in {
    from {
      opacity: 0;
      transform: scale(0.8);
    }
  }
  .name {
    font-size: 24px;
    font-weight: 500;
  }
  .host {
    color: rgb(255 255 255 / 0.6);
  }
  .opts {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 300px;
    padding: 6px;
    border-radius: 14px;
    background: rgb(255 255 255 / 0.08);
    border: 1px solid rgb(255 255 255 / 0.14);
    -webkit-backdrop-filter: blur(20px);
    backdrop-filter: blur(20px);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.08);
  }
  .opts button {
    animation: opt-in 360ms var(--ease-out) both;
    animation-delay: calc(140ms + var(--i, 0) * 45ms);
    display: flex;
    align-items: center;
    gap: 14px;
    height: 46px;
    padding: 0 16px;
    border-radius: 9px;
    color: #fff;
    font-size: var(--fs-md);
    text-align: left;
  }
  .opts button :global(svg) {
    color: rgb(255 255 255 / 0.72);
  }
  .opts button:hover,
  .opts button:focus-visible {
    outline: none;
    background: rgb(255 255 255 / 0.13);
  }
  @keyframes opt-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .cancel {
    animation: opt-in 360ms var(--ease-out) 340ms both;
    height: 40px;
    padding: 0 30px;
    border-radius: 20px;
    color: #fff;
    font-size: var(--fs-md);
    background: rgb(255 255 255 / 0.12);
    border: 1px solid rgb(255 255 255 / 0.22);
  }
  .cancel:hover,
  .cancel:focus-visible {
    outline: none;
    background: rgb(255 255 255 / 0.2);
  }
  footer {
    position: absolute;
    right: 32px;
    bottom: 28px;
  }
  .power-wrap {
    position: relative;
  }
  .power {
    display: grid;
    place-items: center;
    width: 50px;
    height: 50px;
    border-radius: 50%;
    color: #fff;
    background: rgb(255 255 255 / 0.1);
    border: 1px solid rgb(255 255 255 / 0.22);
  }
  .power:hover,
  .power[aria-expanded="true"] {
    background: rgb(255 255 255 / 0.2);
  }
  .power-menu {
    position: absolute;
    right: 0;
    bottom: 60px;
    min-width: 220px;
    padding: 5px;
    border-radius: 12px;
    background: rgb(24 30 40 / 0.88);
    border: 1px solid rgb(255 255 255 / 0.14);
    -webkit-backdrop-filter: blur(24px);
    backdrop-filter: blur(24px);
    box-shadow: 0 18px 44px rgb(0 0 0 / 0.4);
    animation: rise 160ms var(--ease);
  }
  .power-menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 12px;
    border-radius: 7px;
    color: #fff;
    text-align: left;
  }
  .power-menu button:hover {
    background: rgb(255 255 255 / 0.1);
  }
</style>
