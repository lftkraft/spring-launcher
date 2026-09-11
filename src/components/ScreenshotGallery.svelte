<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Image as ImageIcon, Trash2, X, Copy, ZoomIn, ZoomOut, Loader2, Maximize2 } from 'lucide-svelte';
  import { showNotification } from '../stores/notification';
  import { portal } from '../utils/portal';
  import './ScreenshotGallery.css';

  interface ScreenshotInfo {
    name: string;
    path: string;
    created: number;
  }

  let { gameDir } = $props<{ gameDir: string }>();

  let screenshots = $state<ScreenshotInfo[]>([]);
  let loading = $state(true);
  let previews = $state<Record<string, string>>({});
  let selectedImage = $state<string | null>(null);
  let isClosingLightbox = $state(false);
  let zoomLevel = $state(1);
  let panOffset = $state({ x: 0, y: 0 });
  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let imageElement: HTMLImageElement | null = null;

  let panelPos = $state<{ x: number; y: number }>({ x: 50, y: 90 });
  let isDraggingPanel = $state(false);
  let panelDragOffsetX = 0;
  let panelDragOffsetY = 0;

  onMount(() => {
    const saved = localStorage.getItem('lightbox-panel-pos-rel');
    if (saved) {
      try {
        panelPos = JSON.parse(saved);
      } catch (e) {
        console.error(e);
      }
    }
    loadScreenshots();
  });

  $effect(() => {
    if (gameDir) {
      loadScreenshots();
    }
  });

  async function loadScreenshots() {
    try {
      loading = true;
      const result: ScreenshotInfo[] = await invoke('get_screenshots', { gameDir });
      screenshots = result;

      for (const s of result) {
        if (!previews[s.path]) {
          const data: string = await invoke('get_screenshot_full', { path: s.path });
          previews[s.path] = data;
        }
      }
    } catch (err) {
      console.error(err);
    } finally {
      loading = false;
    }
  }

  async function handlePreview(s: ScreenshotInfo) {
    try {
      const data: string = await invoke('get_screenshot_full', { path: s.path });
      selectedImage = data;
      zoomLevel = 1;
      panOffset = { x: 0, y: 0 };
      isClosingLightbox = false;
    } catch (err) {
      showNotification('Hiba a kép betöltésekor', 'error');
    }
  }

  function closeLightbox() {
    isClosingLightbox = true;
    setTimeout(() => {
      selectedImage = null;
      isClosingLightbox = false;
      zoomLevel = 1;
      panOffset = { x: 0, y: 0 };
    }, 300);
  }

  async function handleCopyImage() {
    if (!selectedImage) return;
    try {
      await navigator.clipboard.writeText(selectedImage);
      showNotification('Másolva!', 'success');
    } catch (err) {
      showNotification('Sikertelen', 'error');
    }
  }

  function handleMouseDown(e: MouseEvent) {
    if (zoomLevel > 1) {
      isDragging = true;
      dragStartX = e.clientX - panOffset.x;
      dragStartY = e.clientY - panOffset.y;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging && zoomLevel > 1 && imageElement) {
      const newX = e.clientX - dragStartX;
      const newY = e.clientY - dragStartY;
      const rect = imageElement.getBoundingClientRect();
      const container = imageElement.parentElement;
      if (!container) return;
      const contRect = container.getBoundingClientRect();
      const maxX = rect.width > contRect.width ? (rect.width - contRect.width) / 2 : 0;
      const maxY = rect.height > contRect.height ? (rect.height - contRect.height) / 2 : 0;
      panOffset = {
        x: Math.max(-maxX, Math.min(maxX, newX)),
        y: Math.max(-maxY, Math.min(maxY, newY))
      };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handlePanelMouseDown(e: MouseEvent) {
    isDraggingPanel = true;
    const mouseXPercent = (e.clientX / window.innerWidth) * 100;
    const mouseYPercent = (e.clientY / window.innerHeight) * 100;
    panelDragOffsetX = mouseXPercent - panelPos.x;
    panelDragOffsetY = mouseYPercent - panelPos.y;
    e.stopPropagation();
  }

  function handleGlobalMouseMove(e: MouseEvent) {
    if (isDraggingPanel) {
      const mouseXPercent = (e.clientX / window.innerWidth) * 100;
      const mouseYPercent = (e.clientY / window.innerHeight) * 100;
      panelPos = {
        x: Math.max(5, Math.min(95, mouseXPercent - panelDragOffsetX)),
        y: Math.max(5, Math.min(95, mouseYPercent - panelDragOffsetY))
      };
    } else {
      handleMouseMove(e);
    }
  }

  function handleGlobalMouseUp() {
    if (isDraggingPanel) {
      isDraggingPanel = false;
      localStorage.setItem('lightbox-panel-pos-rel', JSON.stringify(panelPos));
    }
    handleMouseUp();
  }

  async function handleDelete(e: MouseEvent, path: string) {
    e.stopPropagation();
    if (window.confirm('Biztosan törölni szeretnéd?')) {
      try {
        await invoke('delete_file_item', { path });
        screenshots = screenshots.filter((s) => s.path !== path);
        showNotification('Screenshot törölve', 'info');
      } catch (err) {
        showNotification('Hiba a törlés során', 'error');
      }
    }
  }
</script>

<div class="screenshot-gallery fade-in">
  {#if loading && screenshots.length === 0}
    <div class="gallery-loading">
      <Loader2 class="spin" size={32} />
      <p>Screenshotok betöltése...</p>
    </div>
  {:else if screenshots.length === 0}
    <div class="empty-gallery">
      <ImageIcon size={64} />
      <h3>Nincsenek screenshotok</h3>
      <p>Játék közben az F2 billentyűvel készíthetsz képeket.</p>
    </div>
  {:else}
    <div class="screenshot-grid">
      {#each screenshots as s (s.path)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="screenshot-card" onclick={() => handlePreview(s)}>
          <div class="screenshot-preview">
            {#if previews[s.path]}
              <img src={previews[s.path]} alt="" />
            {:else}
              <div class="placeholder-shimmer"></div>
            {/if}
            <div class="screenshot-overlay">
              <button
                class="action-btn delete"
                onclick={(e) => handleDelete(e, s.path)}
                title="Törlés"
              >
                <Trash2 size={18} />
              </button>
              <div class="zoom-hint"><Maximize2 size={24} /></div>
            </div>
          </div>
          <div class="screenshot-info">
            <span class="name">{s.name}</span>
            <span class="date">{new Date(s.created).toLocaleDateString()}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if selectedImage}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={`image-lightbox-overlay fade-in ${isClosingLightbox ? 'closing' : ''}`}
      use:portal
      onmousemove={handleGlobalMouseMove}
      onmouseup={handleGlobalMouseUp}
      onmouseleave={handleGlobalMouseUp}
    >
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="lightbox-image-wrapper" onclick={closeLightbox}>
        <div
          class="lightbox-image-container"
          onclick={(e) => e.stopPropagation()}
          onmousedown={handleMouseDown}
          style={`cursor: ${zoomLevel > 1 ? (isDragging ? 'grabbing' : 'grab') : 'default'}`}
        >
          <img
            bind:this={imageElement}
            src={selectedImage}
            alt=""
            draggable={false}
            style={`transform: translate(${panOffset.x}px, ${panOffset.y}px) scale(${zoomLevel}); transition: ${isDragging ? 'none' : 'transform 0.2s ease-out'}`}
            onwheel={(e) => {
              const newZoom = e.deltaY < 0 ? Math.min(5, zoomLevel + 0.1) : Math.max(1, zoomLevel - 0.1);
              zoomLevel = newZoom;
              if (newZoom === 1) panOffset = { x: 0, y: 0 };
            }}
          />
        </div>
      </div>

      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="lightbox-controls-floating"
        onmousedown={handlePanelMouseDown}
        style={`position: fixed; left: ${panelPos.x}%; top: ${panelPos.y}%; transform: translate(-50%, -50%); cursor: ${isDraggingPanel ? 'grabbing' : 'grab'}`}
        onclick={(e) => e.stopPropagation()}
      >
        <button
          class="ctrl-btn-round"
          onclick={(e) => {
            e.stopPropagation();
            handleCopyImage();
          }}
          title="Másolás"
        >
          <Copy size={20} />
        </button>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="zoom-capsule" onmousedown={(e) => e.stopPropagation()}>
          <button
            class="ctrl-btn-round mini"
            onclick={() => {
              zoomLevel = Math.max(1, zoomLevel - 0.2);
              if (zoomLevel <= 1.2) panOffset = { x: 0, y: 0 };
            }}
          >
            <ZoomOut size={18} />
          </button>
          <span class="zoom-val">{Math.round(zoomLevel * 100)}%</span>
          <button
            class="ctrl-btn-round mini"
            onclick={() => {
              zoomLevel = Math.min(5, zoomLevel + 0.2);
            }}
          >
            <ZoomIn size={18} />
          </button>
        </div>
        <button
          class="ctrl-btn-round danger"
          onclick={(e) => {
            e.stopPropagation();
            closeLightbox();
          }}
          title="Bezárás"
        >
          <X size={20} />
        </button>
      </div>
    </div>
  {/if}
</div>
