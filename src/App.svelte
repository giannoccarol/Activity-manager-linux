<script lang="ts">
  import { onMount } from "svelte";
  import { app, type Page } from "./lib/state.svelte";
  import { subscribe, isTauri } from "./lib/api";
  import { settings } from "./lib/settings.svelte";
  import { ui } from "./lib/ui.svelte";
  import { enterOn } from "./lib/motion.svelte";
  import { applyLanguage, i18n, t } from "./lib/i18n.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Processes from "./pages/Processes.svelte";
  import Performance from "./pages/Performance.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import History from "./pages/History.svelte";
  import Startup from "./pages/Startup.svelte";
  import Network from "./pages/Network.svelte";
  import Users from "./pages/Users.svelte";
  import Details from "./pages/Details.svelte";
  import Services from "./pages/Services.svelte";
  import RunDialog from "./components/RunDialog.svelte";
  import PropsDialog from "./components/PropsDialog.svelte";
  import Toasts from "./components/Toasts.svelte";
  import SecureScreen from "./pages/SecureScreen.svelte";

  // page title keys in the dictionary (also the list of valid pages, in Ctrl+1…9 order)
  const TITLES: Record<Page, string> = {
    processes: "nav.processes",
    performance: "nav.performance",
    history: "nav.history",
    startup: "nav.startup",
    network: "nav.network",
    users: "nav.users",
    details: "nav.details",
    services: "nav.services",
    settings: "nav.settings",
  };
  const ORDER = Object.keys(TITLES) as Page[];
  const view = (window as unknown as { __VIEW__?: string }).__VIEW__ ?? new URLSearchParams(location.search).get("view");
  const SEARCHLESS: Page[] = ["performance", "settings"];

  onMount(() => {
    const q = new URLSearchParams(location.search);
    settings.init().then(() => {
      const t = q.get("theme");
      if (t === "paper" || t === "glass" || t === "graphite") settings.v.theme = t;
      const l = q.get("lang");
      if (l === "it" || l === "en") applyLanguage(l);
    });
    trackMaximized();
    if (q.get("page") && q.get("page")! in TITLES) app.page = q.get("page") as Page;
    if (view !== "secure") subscribe((s) => app.push(s));
    if (isTauri && view !== "secure")
      import("@tauri-apps/api/event").then(({ listen }) => {
        listen<string>("navigate", (e) => e.payload in TITLES && (app.page = e.payload as Page));
        listen("bench", async () => {
          const { runBench } = await import("./lib/bench");
          const { invoke } = await import("@tauri-apps/api/core");
          invoke("bench_report", { text: await runBench() });
        });
      });
  });

  $effect(() => {
    document.documentElement.dataset.theme = view === "secure" ? "glass" : settings.v.theme;
  });

  // window title in the interface language
  $effect(() => {
    i18n.lang;
    if (!isTauri) document.title = t("app.name");
    if (isTauri && view !== "secure") import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().setTitle(t("app.name")).catch(() => {}));
  });

  const kept = $state({ processes: false, performance: false });
  $effect(() => {
    if (app.page === "processes") kept.processes = true;
    if (app.page === "performance") kept.performance = true;
  });

  // maximized or fullscreen window: no rounded corners or outline
  async function trackMaximized() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const w = getCurrentWindow();
    const sync = async () => {
      const full = (await w.isMaximized()) || (await w.isFullscreen());
      document.documentElement.toggleAttribute("data-max", full);
    };
    await sync();
    await w.onResized(sync);
  }

  function key(e: KeyboardEvent) {
    if (view === "secure") return;
    if (e.ctrlKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      app.page = ORDER[Number(e.key) - 1];
    }
  }
</script>

<svelte:window onkeydown={key} />
{#if view === "secure"}
  {#if !isTauri}<div class="devdesk" aria-hidden="true"></div>{/if}
  <SecureScreen />
{:else}
  {#if !isTauri && settings.v.theme === "glass"}
    <!-- dev browser only: simulates the desktop that the app shows blurred by KWin -->
    <div class="devdesk" aria-hidden="true"></div>
  {/if}
  <div class="shell">
    <Sidebar />
    <main>
      <TopBar title={t(TITLES[app.page])} searchable={!SEARCHLESS.includes(app.page)} />
      <div class="content">
        {#key app.page}
        {#if !(app.snap && (app.page === "processes" || app.page === "performance"))}
        <div class="view view-in">
        {#if app.page === "settings"}
          <SettingsPage />
        {:else if app.page === "history"}
          <History />
        {:else if app.page === "startup"}
          <Startup />
        {:else if app.page === "services"}
          <Services />
        {:else if !app.snap}
          <p class="wait">{t("common.collecting")}</p>
        {:else if app.page === "processes" || app.page === "performance"}
          <!-- the two most used pages stay in memory: going back to them is instant -->
        {:else if app.page === "network"}
          <Network />
        {:else if app.page === "users"}
          <Users />
        {:else if app.page === "details"}
          <Details />
        {/if}
        </div>
        {/if}
        {/key}
        {#if app.snap}
          {#if kept.processes}<div class="kept view-in" class:off={app.page !== "processes"} use:enterOn={app.page === "processes"}><Processes /></div>{/if}
          {#if kept.performance}<div class="kept view-in" class:off={app.page !== "performance"} use:enterOn={app.page === "performance"}><Performance /></div>{/if}
        {/if}
      </div>
    </main>
  </div>
  {#if ui.dialog?.kind === "run"}<RunDialog />{:else if ui.dialog?.kind === "props"}<PropsDialog data={ui.dialog.data} />{/if}
  <Toasts />
{/if}

<style>
  .shell {
    position: relative;
    display: grid;
    grid-template-columns: 216px minmax(0, 1fr);
    height: 100%;
    overflow: hidden;
    background: var(--canvas);
    border-radius: var(--win-r);
    box-shadow: inset 0 0 0 1px var(--win-line);
  }
  :global(html[data-max]) .shell {
    border-radius: 0;
    box-shadow: none;
  }
  .devdesk {
    position: fixed;
    inset: 0;
    background:
      radial-gradient(circle at 18% 22%, #2f6f9f 0, transparent 38%),
      radial-gradient(circle at 78% 30%, #3a8a86 0, transparent 34%),
      radial-gradient(circle at 60% 85%, #8a6a3a 0, transparent 36%),
      #223040;
    filter: blur(40px);
  }
  .shell > :global(nav) {
    background: var(--side-bg);
  }
  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--main-bg);
  }
  .content {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .view,
  .kept {
    position: absolute;
    inset: 0;
  }

  /* page kept in memory: not painted or clickable, but with its layout ready */
  .kept.off {
    content-visibility: hidden;
    pointer-events: none;
    visibility: hidden;
    z-index: -1;
  }
  .wait {
    margin: 80px auto;
    text-align: center;
    color: var(--muted);
  }
</style>
