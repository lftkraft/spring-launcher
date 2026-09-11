<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { Trash2, Play, Plus, Search, MoreHorizontal, FolderOpen, Info, Copy, Download } from 'lucide-svelte';
  import CreateInstanceModal from '../../components/CreateInstanceModal.svelte';
  import { showNotification } from '../../stores/notification';
  import './InstancesPage.css';

  interface InstanceDisplay {
    id: string;
    name: string;
    mc_version: string;
    loader: string;
    memory: number;
    icon?: string;
    last_played?: string;
    created: string;
    game_dir?: string;
    playtime?: number;
  }

  let instances = $state<InstanceDisplay[]>([]);
  let searchQuery = $state('');
  let showCreateModal = $state(false);
  let duplicateTarget = $state<InstanceDisplay | null>(null);
  let loading = $state(true);
  let activeMenu = $state<string | null>(null);
  let hoverInfo = $state<string | null>(null);
  let unlistenUpdated: (() => void) | undefined;

  async function loadInstances() {
    try {
      loading = true;
      const data: any[] = await invoke('get_instances');
      const mapped = data.map((i) => ({
        ...i,
        mc_version: i.mc_version || i.mcVersion,
        last_played: i.last_played || i.lastPlayed
      }));
      instances = mapped;
    } catch (err) {
      console.error('Failed to load instances:', err);
    } finally {
      loading = false;
    }
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.more-actions-wrapper')) {
      activeMenu = null;
    }
  }

  onMount(async () => {
    loadInstances();
    document.addEventListener('mousedown', handleClickOutside);
    unlistenUpdated = await listen('instances-updated', () => {
      loadInstances();
    });
  });

  onDestroy(() => {
    document.removeEventListener('mousedown', handleClickOutside);
    if (unlistenUpdated) unlistenUpdated();
  });

  let deletingId: string | null = $state(null);

  async function deleteInstance(id: string, name: string) {
    if (deletingId) return;
    if (!confirm(`Biztosan törölni szeretnéd a(z) "${name}" instance-t?`)) return;
    deletingId = id;
    showNotification(`"${name}" törlése folyamatban...`, 'info');
    try {
      await invoke('delete_instance', { id });
      showNotification('Instance sikeresen törölve', 'success');
      loadInstances();
    } catch (err) {
      showNotification('Hiba a törlés során', 'error');
    } finally {
      deletingId = null;
    }
  }

  async function openFolder(path?: string) {
    if (path) {
      await invoke('open_folder', { path });
    }
    activeMenu = null;
  }

  function formatPlaytime(seconds?: number) {
    if (!seconds) return '0 perc';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (h > 0) return `${h} óra ${m} perc`;
    return `${m} perc`;
  }

  let filteredInstances = $derived(
    instances.filter(
      (i) =>
        i.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        i.mc_version.includes(searchQuery)
    )
  );
</script>

<div class="instances-page page-container animate-fade-in">
  <div class="instances-header">
    <div class="header-title">
      <h1>Instance Kezelő</h1>
      <p>{instances.length} telepített verzió</p>
    </div>

    <div class="search-bar-container">
      <div class="search-bar">
        <Search size={18} />
        <input
          type="text"
          placeholder="Keress az instance-id között..."
          bind:value={searchQuery}
        />
      </div>
    </div>

    <div class="header-actions">
      <button class="new-instance-btn" onclick={() => { duplicateTarget = null; showCreateModal = true; }} title="Új Instance létrehozása">
        <Plus size={28} />
      </button>
    </div>
  </div>

  <div class="instances-grid">
    {#if loading}
      <div class="loading-placeholder">Instance-ek betöltése...</div>
    {:else if filteredInstances.length === 0}
      <div class="empty-state">
        <Search size={64} />
        <h3>Nincs találat</h3>
        <p>Próbálj meg más keresési kifejezést vagy hozz létre egy új instance-t.</p>
      </div>
    {:else}
      {#each filteredInstances as instance (instance.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="instance-manage-card"
          class:menu-open={activeMenu === instance.id}
          onclick={() => goto(`/instance/${instance.id}`)}
        >
          <button
            class="card-info-trigger"
            onmouseenter={() => (hoverInfo = instance.id)}
            onmouseleave={() => (hoverInfo = null)}
            onclick={(e) => e.stopPropagation()}
            aria-label="Info"
          >
            <Info size={18} />
          </button>

          {#if hoverInfo === instance.id}
            <div class="instance-popover fade-in">
              <div class="popover-item">
                <span class="popover-label">Létrehozva</span>
                <span class="popover-value">{new Date(instance.created).toLocaleDateString()}</span>
              </div>
              <div class="popover-item">
                <span class="popover-label">Utoljára játszva</span>
                <span class="popover-value">
                  {instance.last_played ? new Date(instance.last_played).toLocaleDateString() : 'Soha'}
                </span>
              </div>
              <div class="popover-item">
                <span class="popover-label">Játékidő</span>
                <span class="popover-value">{formatPlaytime(instance.playtime)}</span>
              </div>
            </div>
          {/if}

          <div class="card-icon">
            {#if instance.icon}
              <img src={instance.icon} alt={instance.name} />
            {:else}
              <Play size={44} fill="currentColor" />
            {/if}
          </div>

          <div class="card-info">
            <h3>{instance.name}</h3>
            <div class="card-meta">
              <span class="meta-badge">{instance.mc_version}</span>
              <span class="meta-badge">{instance.loader}</span>
            </div>
          </div>

          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="card-actions" onclick={(e) => e.stopPropagation()}>
            <div class="more-actions-wrapper" style="position: relative;">
              <button
                class={`icon-btn more ${activeMenu === instance.id ? 'active' : ''}`}
                onclick={() => (activeMenu = activeMenu === instance.id ? null : instance.id)}
                aria-label="More options"
              >
                <MoreHorizontal size={20} />
              </button>

              {#if activeMenu === instance.id}
                <div class="actions-dropdown fade-in">
                  <button
                    class="dropdown-item"
                    onclick={() => {
                      duplicateTarget = instance;
                      activeMenu = null;
                      showCreateModal = true;
                    }}
                  >
                    <Copy size={16} />
                    <span>Duplikálás</span>
                  </button>
                  <button
                    class="dropdown-item"
                    onclick={() => {
                      activeMenu = null;
                      goto(`/mods?instance=${instance.id}&version=${encodeURIComponent(instance.mc_version)}&loader=${encodeURIComponent(instance.loader)}`);
                    }}
                  >
                    <Download size={16} />
                    <span>Modok böngészése</span>
                  </button>
                  <button class="dropdown-item" onclick={() => openFolder(instance.game_dir)}>
                    <FolderOpen size={16} />
                    <span>Mappa megnyitása</span>
                  </button>
                  <button
                    class="dropdown-item danger"
                    onclick={() => deleteInstance(instance.id, instance.name)}
                  >
                    <Trash2 size={16} />
                    <span>Törlés</span>
                  </button>
                </div>
              {/if}
            </div>

            <button
              class="icon-btn play"
              onclick={() => goto(`/instance/${instance.id}`)}
            >
              <Play size={18} fill="currentColor" />
              <span style="margin-left: 8px; font-weight: bold;">JÁTÉK</span>
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  {#if showCreateModal}
    <CreateInstanceModal
      duplicateSource={duplicateTarget}
      onClose={() => {
        showCreateModal = false;
        duplicateTarget = null;
      }}
      onCreated={() => {
        showCreateModal = false;
        duplicateTarget = null;
        loadInstances();
      }}
    />
  {/if}
</div>
