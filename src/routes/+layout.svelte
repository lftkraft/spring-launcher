<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { listen } from '@tauri-apps/api/event';
  import TitleBar from '../components/TitleBar.svelte';
  import SideBar from '../components/SideBar.svelte';
  import NotificationContainer from '../components/NotificationContainer.svelte';
  import UpdaterModal from '../components/UpdaterModal.svelte';
  import '../styles/global.css';

  let { children } = $props();

  let instancesLoaded = $state(false);
  let isSplashActive = $state(false);
  let isSplashComplete = $state(false);
  let isFocused = $state(true);
  let hasSplashRun = false;
  let unlistenLauncherDebug: (() => void) | undefined;

  let timer1: ReturnType<typeof setTimeout> | null = null;
  let timer2: ReturnType<typeof setTimeout> | null = null;

  function handleFocus() {
    isFocused = true;
  }

  function handleBlur() {
    isFocused = false;
  }

  function handleInstancesLoaded() {
    if (hasSplashRun) {
      instancesLoaded = true;
      isSplashComplete = true;
      return;
    }
    hasSplashRun = true;
    instancesLoaded = true;

    timer1 = setTimeout(() => {
      isSplashActive = true;
      timer2 = setTimeout(() => {
        isSplashActive = false;
        isSplashComplete = true;
      }, 5500);
    }, 500);
  }

  let fallbackTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    window.addEventListener('focus', handleFocus);
    window.addEventListener('blur', handleBlur);
    window.addEventListener('instances-loaded', handleInstancesLoaded);

    fallbackTimer = setTimeout(() => {
      if (!instancesLoaded) {
        instancesLoaded = true;
        isSplashComplete = true;
      }
    }, 1000);

    unlistenLauncherDebug = await listen<string>('launcher-debug', (event) => {
      console.log('[launcher-debug]', event.payload);
    });
  });

  onDestroy(() => {
    window.removeEventListener('focus', handleFocus);
    window.removeEventListener('blur', handleBlur);
    window.removeEventListener('instances-loaded', handleInstancesLoaded);
    if (unlistenLauncherDebug) unlistenLauncherDebug();
    if (fallbackTimer) clearTimeout(fallbackTimer);
    if (timer1) clearTimeout(timer1);
    if (timer2) clearTimeout(timer2);
  });

  let isConsole = $derived($page.url.pathname === '/console');
</script>

<NotificationContainer />
<UpdaterModal />

{#if isConsole}
  {@render children()}
{:else}
  <div class={`app-container ${instancesLoaded ? 'loaded' : 'loading'} ${!isFocused ? 'unfocused' : ''}`}>
    <SideBar {isFocused} />

    <div class="main-view">
      <TitleBar
        {isSplashComplete}
        {isSplashActive}
        {isFocused}
      />

      <div class="app-content">
        {@render children()}
      </div>
    </div>
  </div>
{/if}
