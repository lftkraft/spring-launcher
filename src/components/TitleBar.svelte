<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Minus, Leaf, Sparkles } from 'lucide-svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { availableUpdate, showUpdaterModal } from '../stores/updater';
  import './TitleBar.css';

  let {
    isSplashComplete = false,
    isSplashActive = false,
    isFocused = true
  } = $props<{
    isSplashComplete: boolean;
    isSplashActive: boolean;
    isFocused: boolean;
  }>();

  let isHovered = $state(false);
  let splashPhase = $state<'idle' | 'bar' | 'circle' | 'fadeout' | 'collapse' | 'done'>('idle');
  let forceExpanded = $state(false);
  let isHolding = $state(false);
  let holdTimeout: ReturnType<typeof setTimeout> | null = null;
  let unlistenResize: (() => void) | undefined;

  let timer1: ReturnType<typeof setTimeout> | null = null;
  let timer2: ReturnType<typeof setTimeout> | null = null;
  let timer3: ReturnType<typeof setTimeout> | null = null;
  let timer4: ReturnType<typeof setTimeout> | null = null;

  function triggerHold() {
    isHolding = true;
    forceExpanded = true;
    if (holdTimeout) clearTimeout(holdTimeout);
    holdTimeout = setTimeout(() => {
      isHolding = false;
      forceExpanded = false;
    }, 2000);
  }

  onMount(async () => {
    try {
      const win = getCurrentWindow();
      unlistenResize = await win.onResized(() => {
        triggerHold();
      });
    } catch (e) {
      console.warn('Failed to attach window resize listener:', e);
    }
  });

  onDestroy(() => {
    if (unlistenResize) unlistenResize();
    if (holdTimeout) clearTimeout(holdTimeout);
    if (timer1) clearTimeout(timer1);
    if (timer2) clearTimeout(timer2);
    if (timer3) clearTimeout(timer3);
    if (timer4) clearTimeout(timer4);
  });

  $effect(() => {
    if (!isSplashActive) {
      if (isSplashComplete) splashPhase = 'done';
      return;
    }

    forceExpanded = true;
    splashPhase = 'bar';

    timer1 = setTimeout(() => { splashPhase = 'circle'; }, 2000);
    timer2 = setTimeout(() => { splashPhase = 'fadeout'; }, 3000);
    timer3 = setTimeout(() => { splashPhase = 'collapse'; }, 4000);
    timer4 = setTimeout(() => {
      splashPhase = 'done';
      if (!isHolding) forceExpanded = false;
    }, 5000);
  });

  async function handleMinimize() {
    try {
      const win = getCurrentWindow();
      await win.minimize();
    } catch (e) {
      console.error(e);
    }
  }

  async function handleMaximize() {
    triggerHold();
    try {
      const win = getCurrentWindow();
      await win.toggleMaximize();
    } catch (e) {
      console.error(e);
    }
  }

  async function handleClose() {
    try {
      const win = getCurrentWindow();
      await win.close();
    } catch (e) {
      console.error(e);
    }
  }

  let isExpanded = $derived(isHovered || forceExpanded || isHolding);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={`titlebar ${isExpanded ? 'expanded' : 'collapsed'} ${!isFocused ? 'unfocused' : ''} splash-${splashPhase} ${isHolding ? 'holding' : ''}`}
  onmouseenter={() => (isHovered = true)}
  onmouseleave={() => (isHovered = false)}
>
  <div class="titlebar-left">
    <Leaf class="titlebar-icon" />
    <span class="titlebar-spring">Spring</span>
    <span class="titlebar-launcher">launcher</span>
  </div>

  <div class="titlebar-right">
    {#if $availableUpdate}
      <button
        class="titlebar-update-badge"
        onclick={() => showUpdaterModal.set(true)}
        title={`Frissítés érhető el: v${$availableUpdate.version}`}
      >
        <Sparkles size={14} class="badge-sparkle-icon" />
        <span>v{$availableUpdate.version}</span>
      </button>
    {/if}
    <button class="titlebar-btn minimize" onclick={handleMinimize} title="Minimize">
      <Minus size={20} />
    </button>
    <button class="titlebar-btn maximize" onclick={handleMaximize} title="Maximize">
      <svg width="14" height="14" viewBox="0 0 448 512" fill="currentColor">
        <path d="M400 32H48C21.5 32 0 53.5 0 80v352c0 26.5 21.5 48 48 48h352c26.5 0 48-21.5 48-48V80c0-26.5-21.5-48-48-48z" />
      </svg>
    </button>
    <button class="titlebar-btn close" onclick={handleClose} title="Close">
      <svg width="14" height="14" viewBox="0 0 352 512" fill="currentColor">
        <path d="M242.72 256l100.07-100.07c12.28-12.28 12.28-32.19 0-44.48l-22.24-22.24c-12.28-12.28-32.19-12.28-44.48 0L176 189.28 75.93 89.21c-12.28-12.28-32.19-12.28-44.48 0L9.21 111.45c-12.28 12.28-12.28 32.19 0 44.48L109.28 256 9.21 356.07c-12.28 12.28-12.28 32.19 0 44.48l22.24 22.24c12.28 12.28 32.19 12.28 44.48 0L176 322.72l100.07 100.07c12.28 12.28 32.19 12.28 44.48 0l22.24-22.24c12.28-12.28 12.28-32.19 0-44.48L242.72 256z" />
      </svg>
    </button>
  </div>

  {#if splashPhase !== 'idle' && splashPhase !== 'done'}
    <div class="titlebar-splash">
      <div class="titlebar-splash-bar"></div>
    </div>
  {/if}
</div>
