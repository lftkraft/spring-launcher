<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { X, Box, Check, ChevronLeft, ChevronDown, Cpu, Layout, Image as ImageIcon, Loader2 } from 'lucide-svelte';
  import { portal } from '../utils/portal';
  import './CreateInstanceModal.css';

  type Step = 'loader' | 'details';

  let { onClose, onCreated, duplicateSource = null } = $props<{
    onClose: () => void;
    onCreated: (created?: any) => void;
    duplicateSource?: any;
  }>();

  let step = $state<Step>('loader');
  let name = $state('');
  let mcVersion = $state('');
  let loader = $state('vanilla');
  let loaderVersion = $state('');
  let memory = $state(4096);
  let icon = $state<string | null>(null);

  $effect(() => {
    if (duplicateSource) {
      step = 'details';
      name = `${duplicateSource.name} (Másolat)`;
      mcVersion = duplicateSource.mcVersion || duplicateSource.mc_version || '';
      loader = (duplicateSource.loader || 'vanilla').toLowerCase();
      loaderVersion = duplicateSource.loaderVersion || duplicateSource.loader_version || '';
      memory = duplicateSource.memory || 4096;
      icon = duplicateSource.icon || null;
    }
  });

  let mcVersions = $state<string[]>([]);
  let loaderVersions = $state<string[]>([]);
  let loading = $state(true);
  let loadersLoading = $state(false);
  let isClosing = $state(false);
  let error = $state<string | null>(null);
  let showVersionDropdown = $state(false);

  onMount(async () => {
    loadMcVersions();
    if (!duplicateSource) {
      try {
        const settings: any = await invoke('get_settings');
        if (settings?.default_memory) {
          memory = settings.default_memory;
        }
      } catch (e) {
        console.error(e);
      }
    }
  });

  $effect(() => {
    document.body.classList.add('modal-open');
    return () => {
      document.body.classList.remove('modal-open');
    };
  });

  $effect(() => {
    if (mcVersion && loader !== 'vanilla') {
      loadLoaderVersions();
    }
  });

  async function loadMcVersions() {
    try {
      loading = true;
      const versions: string[] = await invoke('get_minecraft_versions');
      mcVersions = versions;
      if (!mcVersion && versions.length > 0) mcVersion = versions[0];
    } catch (err) {
      error = 'Hiba a verziók betöltésekor';
    } finally {
      loading = false;
    }
  }

  async function loadLoaderVersions() {
    try {
      loadersLoading = true;
      const versions: string[] = await invoke('get_loader_versions', { loader, mcVersion });
      loaderVersions = versions;
      if (versions.length > 0) {
        if (!loaderVersion || !loaderVersions.includes(loaderVersion)) {
          loaderVersion = versions[0];
        }
      }
    } catch (err) {
      console.error(err);
    } finally {
      loadersLoading = false;
    }
  }

  function handleClose() {
    isClosing = true;
    setTimeout(onClose, 300);
  }

  async function selectIcon() {
    try {
      const path = await invoke<string | null>('select_file', {
        title: 'Válassz ikont',
        filterName: 'Képek',
        filterExt: 'png'
      });
      if (path) {
        const base64 = await invoke<string>('get_screenshot_full', { path });
        icon = base64;
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) {
      error = 'Adj meg egy nevet!';
      return;
    }

    try {
      loading = true;
      const memVal = Number(memory) || 4096;
      let res;
      if (duplicateSource) {
        res = await invoke('duplicate_instance', {
          sourceId: duplicateSource.id,
          name,
          mcVersion,
          loader,
          loaderVersion: loader !== 'vanilla' ? loaderVersion : null,
          memory: memVal,
          icon
        });
      } else {
        res = await invoke('create_instance', {
          name,
          mcVersion,
          loader,
          loaderVersion: loader !== 'vanilla' ? loaderVersion : null,
          memory: memVal,
          icon
        });
      }
      onCreated(res);
    } catch (err) {
      error = String(err);
      loading = false;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class={`create-modal-overlay ${isClosing ? 'closing' : ''}`} use:portal onclick={handleClose}>
  <div class="create-modal-content" onclick={(e) => e.stopPropagation()}>
    <button class="close-btn" onclick={handleClose} aria-label="Close"><X size={20} /></button>

    {#if step === 'loader'}
      <div class="modal-step fade-in">
        <h2>Válassz típust</h2>
        <p class="subtitle">Milyen Minecraftot szeretnél telepíteni?</p>

        <div class="loader-grid">
          <div
            class="loader-card"
            onclick={() => {
              loader = 'vanilla';
              step = 'details';
            }}
          >
            <div class="loader-icon vanilla">
              <img src="/loaders/vanilla.svg" alt="Vanilla" class="loader-img" />
            </div>
            <div class="loader-info">
              <h3>Vanilla</h3>
              <p>Módosítások nélküli, gyári élmény.</p>
            </div>
            <Check size={20} class="check-icon" />
          </div>

          <div
            class="loader-card"
            onclick={() => {
              loader = 'neoforge';
              step = 'details';
            }}
          >
            <div class="loader-icon neoforge">
              <img src="/loaders/neoforge.svg" alt="NeoForge" class="loader-img" />
            </div>
            <div class="loader-info">
              <h3>NeoForge</h3>
              <p>Új generációs modbetöltő a modern verziókhoz.</p>
            </div>
            <Check size={20} class="check-icon" />
          </div>

          <div
            class="loader-card"
            onclick={() => {
              loader = 'forge';
              step = 'details';
            }}
          >
            <div class="loader-icon forge">
              <img src="/loaders/forge.png" alt="Forge" class="loader-img" />
            </div>
            <div class="loader-info">
              <h3>Forge</h3>
              <p>A legnépszerűbb klasszikus modbetöltő.</p>
            </div>
            <Check size={20} class="check-icon" />
          </div>

          <div
            class="loader-card"
            onclick={() => {
              loader = 'fabric';
              step = 'details';
            }}
          >
            <div class="loader-icon fabric">
              <img src="/loaders/fabric.svg" alt="Fabric" class="loader-img" />
            </div>
            <div class="loader-info">
              <h3>Fabric</h3>
              <p>Könnyed, modern és gyors modbetöltő.</p>
            </div>
            <Check size={20} class="check-icon" />
          </div>

          <div
            class="loader-card"
            onclick={() => {
              loader = 'quilt';
              step = 'details';
            }}
          >
            <div class="loader-icon quilt">
              <img src="/loaders/quilt.svg" alt="Quilt" class="loader-img" />
            </div>
            <div class="loader-info">
              <h3>Quilt</h3>
              <p>Moduláris, Fabric-kompatibilis modbetöltő.</p>
            </div>
            <Check size={20} class="check-icon" />
          </div>
        </div>
      </div>
    {:else}
      <div class="modal-step fade-in">
        <button class="back-link" onclick={() => (step = 'loader')}>
          <ChevronLeft size={16} /> Vissza a típusokhoz
        </button>
        <h2>{duplicateSource ? 'Instance duplikálása' : 'Részletek'}</h2>
        <p class="subtitle">{duplicateSource ? 'Állítsd be a másolat adatait. A modok, mentések és konfigurációk másolásra kerülnek.' : 'Állítsd be az instance paramétereit.'}</p>

        <form onsubmit={handleSubmit} class="details-form">
          <div class="icon-upload-section">
            <div class="icon-preview">
              {#if icon}
                <img src={icon} alt="" />
              {:else}
                <ImageIcon size={32} />
              {/if}
              <div class="icon-overlay" onclick={selectIcon}>Módosítás</div>
            </div>
          </div>

          <div class="input-field">
            <label for="inst-name">Instance neve</label>
            <input
              id="inst-name"
              type="text"
              placeholder="Pl: Kalandos 1.21"
              bind:value={name}
              autofocus
            />
          </div>

          <div class="form-row">
            <div class="input-field flex-1">
              <label for="inst-version">Minecraft verzió</label>
              <div class="version-selector-container">
                <div
                  id="inst-version"
                  class={`version-dropdown-trigger ${showVersionDropdown ? 'active' : ''}`}
                  onclick={() => (showVersionDropdown = !showVersionDropdown)}
                >
                  <span>{mcVersion}</span>
                  <ChevronDown size={18} />
                </div>
                {#if showVersionDropdown}
                  <div class="version-dropdown-menu">
                    {#each mcVersions as v}
                      <div
                        class={`version-dropdown-item ${mcVersion === v ? 'selected' : ''}`}
                        onclick={() => {
                          mcVersion = v;
                          showVersionDropdown = false;
                        }}
                      >
                        {v} {#if mcVersion === v}<Check size={14} />{/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>

            {#if loader !== 'vanilla'}
              <div class="input-field flex-1 animate-slide-down">
                <label for="loader-version">{loader.charAt(0).toUpperCase() + loader.slice(1)} verzió</label>
                <div class="version-selector-container">
                  <select
                    id="loader-version"
                    class="version-dropdown-trigger"
                    style="width: 100%; appearance: auto;"
                    bind:value={loaderVersion}
                    disabled={loadersLoading}
                  >
                    {#if loadersLoading}
                      <option>Betöltés...</option>
                    {:else}
                      {#each loaderVersions as v}
                        <option value={v}>{v}</option>
                      {/each}
                    {/if}
                  </select>
                </div>
              </div>
            {/if}
          </div>

          <div class="memory-control-wrapper">
            <div class="memory-header">
              <div class="memory-label-group">
                <Cpu size={18} />
                <label for="inst-memory-input">Memória (RAM)</label>
              </div>
              <div class="memory-display-box">
                <input
                  id="inst-memory-input"
                  type="number"
                  class="memory-number-input"
                  bind:value={memory}
                  min={1024}
                  max={65536}
                  step={512}
                />
                <span class="memory-unit">MB</span>
                <span class="memory-gb-pill">{(memory / 1024).toFixed(1)} GB</span>
              </div>
            </div>

            <div class="range-slider-wrapper">
              <div class="slider-track-container">
                <div
                  class="slider-thumb-tooltip"
                  style="left: calc({Math.min(100, Math.max(0, ((memory - 1024) / (32768 - 1024)) * 100))}% + {(0.5 - Math.min(1, Math.max(0, (memory - 1024) / (32768 - 1024)))) * 22}px);"
                >
                  <span class="tooltip-gb">{(memory / 1024).toFixed(1)} GB</span>
                  <span class="tooltip-mb">({memory} MB)</span>
                </div>
                <input
                  id="inst-memory"
                  type="range"
                  class="ram-slider"
                  bind:value={memory}
                  min={1024}
                  max={32768}
                  step={512}
                  style="background: linear-gradient(to right, var(--accent-green) 0%, var(--accent-green) {Math.min(100, Math.max(0, ((memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) {Math.min(100, Math.max(0, ((memory - 1024) / (32768 - 1024)) * 100))}%, rgba(255, 255, 255, 0.1) 100%)"
                />
              </div>

              <div class="slider-marks-container">
                {#each [
                  { mb: 1024, label: '1 GB' },
                  { mb: 4096, label: '4 GB' },
                  { mb: 8192, label: '8 GB' },
                  { mb: 16384, label: '16 GB' },
                  { mb: 24576, label: '24 GB' },
                  { mb: 32768, label: '32 GB' }
                ] as mark}
                  {@const markPct = ((mark.mb - 1024) / (32768 - 1024)) * 100}
                  <button
                    type="button"
                    class="slider-mark-btn"
                    style="left: calc({markPct}% + {(0.5 - markPct / 100) * 22}px);"
                    class:active={Math.abs(memory - mark.mb) < 256}
                    onclick={() => memory = mark.mb}
                    title="{mark.label} ({mark.mb} MB)"
                  >
                    <span class="mark-pip"></span>
                    <span class="mark-text">{mark.label}</span>
                  </button>
                {/each}
              </div>
            </div>
          </div>

          {#if error}
            <p class="error-msg">{error}</p>
          {/if}

          <button type="submit" class="btn btn-primary create-btn" disabled={loading}>
            {#if loading}
              <Loader2 class="spin" size={18} />
            {:else}
              <Check size={18} />
            {/if}
            <span>{duplicateSource ? 'Instance duplikálása' : 'Instance létrehozása'}</span>
          </button>
        </form>
      </div>
    {/if}
  </div>
</div>
