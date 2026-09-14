<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { LayoutDashboard, Settings as SettingsIcon, Users, Plus, Folder, List, Box, Copy, FolderOpen, Download } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import CreateInstanceModal from './CreateInstanceModal.svelte';
  import SettingsModal from './SettingsModal.svelte';
  import { portal } from '../utils/portal';
  import { t } from '../stores/i18n';
  import './SideBar.css';

  interface InstanceItem {
    id: string;
    name: string;
    icon?: string;
    last_played?: string;
    mc_version?: string;
    loader?: string;
    loader_version?: string;
    memory?: number;
    game_dir?: string;
  }

  let { isFocused = true } = $props<{ isFocused: boolean }>();

  let showCreateModal = $state(false);
  let showSettingsModal = $state(false);
  let duplicateTarget = $state<InstanceItem | null>(null);
  let contextMenu = $state<{ x: number; y: number; instance: InstanceItem } | null>(null);
  let instances = $state<InstanceItem[]>([]);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  async function loadInstances() {
    try {
      const data: InstanceItem[] = await invoke('get_instances');
      const sorted = [...data]
        .sort((a, b) => {
          if (!a.last_played) return 1;
          if (!b.last_played) return -1;
          return new Date(b.last_played).getTime() - new Date(a.last_played).getTime();
        })
        .slice(0, 3);
      instances = sorted;
    } catch (err) {
      console.error('Failed to load instances in sidebar:', err);
    }
  }

  function handleContextMenu(e: MouseEvent, instance: InstanceItem) {
    e.preventDefault();
    contextMenu = {
      x: e.clientX,
      y: e.clientY,
      instance
    };
  }

  function handleGlobalClick() {
    if (contextMenu) {
      contextMenu = null;
    }
  }

  function handleGlobalContextMenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.sidebar-item.instance-item')) {
      contextMenu = null;
    }
  }

  async function openInstanceFolder(path?: string) {
    if (path) {
      await invoke('open_folder', { path });
    }
    contextMenu = null;
  }

  let unlistenUpdated: (() => void) | undefined;

  onMount(async () => {
    loadInstances();
    intervalId = setInterval(loadInstances, 5000);
    window.addEventListener('click', handleGlobalClick);
    window.addEventListener('contextmenu', handleGlobalContextMenu);
    unlistenUpdated = await listen('instances-updated', () => {
      loadInstances();
    });
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
    window.removeEventListener('click', handleGlobalClick);
    window.removeEventListener('contextmenu', handleGlobalContextMenu);
    if (unlistenUpdated) unlistenUpdated();
  });
</script>

<div class={`sidebar ${!isFocused ? 'unfocused' : ''}`}>
  <div class="sidebar-nav">
    <a
      href="/"
      class={`sidebar-item ${$page.url.pathname === '/' ? 'active' : ''}`}
      title={$t('nav.dashboard')}
    >
      <LayoutDashboard size={20} />
      <span>{$t('nav.dashboard')}</span>
    </a>

    <a
      href="/instances"
      class={`sidebar-item ${$page.url.pathname === '/instances' ? 'active' : ''}`}
      title={$t('nav.instances')}
    >
      <List size={20} />
      <span>{$t('nav.instances')}</span>
    </a>

    <a
      href="/profiles"
      class={`sidebar-item ${$page.url.pathname === '/profiles' ? 'active' : ''}`}
      title={$t('nav.profiles')}
    >
      <Users size={20} />
      <span>{$t('nav.profiles')}</span>
    </a>

    <a
      href="/mods"
      class={`sidebar-item ${$page.url.pathname === '/mods' ? 'active' : ''}`}
      title={$t('nav.mods')}
    >
      <Box size={20} />
      <span>{$t('nav.mods')}</span>
    </a>
  </div>

  <div class="sidebar-divider"></div>

  <div class="sidebar-section-label">
    <span>{$t('sidebar.recent')}</span>
    <button class="add-instance-small-btn" onclick={() => (showCreateModal = true)} title={$t('sidebar.newInstance')}>
      <Plus size={14} />
    </button>
  </div>

  <div class="sidebar-nav instances-list">
    {#each instances as instance (instance.id)}
      <a
        href={`/instance/${instance.id}`}
        class={`sidebar-item instance-item ${$page.url.pathname === `/instance/${instance.id}` ? 'active' : ''}`}
        title={instance.name}
        oncontextmenu={(e) => handleContextMenu(e, instance)}
      >
        <div class="instance-icon-small">
          {#if instance.icon}
            <img src={instance.icon} alt="" />
          {:else}
            <Folder size={16} />
          {/if}
        </div>
        <span>{instance.name}</span>
      </a>
    {/each}

    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="sidebar-item add-btn" onclick={() => (showCreateModal = true)}>
      <Plus size={20} />
      <span>{$t('sidebar.newInstance')}</span>
    </div>
  </div>

  <div class="sidebar-divider"></div>

  <div class="sidebar-footer">
    <button
      type="button"
      class={`sidebar-item ${showSettingsModal || $page.url.pathname === '/settings' ? 'active' : ''}`}
      onclick={() => (showSettingsModal = true)}
      title={$t('sidebar.settings')}
    >
      <SettingsIcon size={20} />
      <span>{$t('sidebar.settings')}</span>
    </button>
  </div>
</div>

{#if contextMenu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="sidebar-context-menu fade-in"
    use:portal
    style={`top: ${contextMenu.y}px; left: ${contextMenu.x}px;`}
    onclick={(e) => e.stopPropagation()}
  >
    <button
      class="context-menu-item"
      onclick={() => {
        duplicateTarget = contextMenu?.instance || null;
        contextMenu = null;
      }}
    >
      <Copy size={15} />
      <span>{$t('sidebar.duplicate')}</span>
    </button>
    <button
      class="context-menu-item"
      onclick={() => {
        const inst = contextMenu?.instance;
        contextMenu = null;
        if (inst) {
          const params = new URLSearchParams();
          if (inst.loader && inst.loader.toLowerCase() !== 'vanilla') params.set('loader', inst.loader.toLowerCase());
          if (inst.mc_version) params.set('version', inst.mc_version);
          params.set('instance', inst.id);
          goto(`/mods?${params.toString()}`);
        }
      }}
    >
      <Download size={15} />
      <span>{$t('sidebar.browseMods')}</span>
    </button>
    {#if contextMenu.instance.game_dir}
      <button
        class="context-menu-item"
        onclick={() => openInstanceFolder(contextMenu?.instance.game_dir)}
      >
        <FolderOpen size={15} />
        <span>{$t('sidebar.openFolder')}</span>
      </button>
    {/if}
  </div>
{/if}

{#if showCreateModal}
  <CreateInstanceModal
    onClose={() => (showCreateModal = false)}
    onCreated={() => {
      showCreateModal = false;
      loadInstances();
    }}
  />
{/if}

{#if duplicateTarget}
  <CreateInstanceModal
    duplicateSource={duplicateTarget}
    onClose={() => (duplicateTarget = null)}
    onCreated={() => {
      duplicateTarget = null;
      loadInstances();
    }}
  />
{/if}

{#if showSettingsModal}
  <SettingsModal
    onClose={() => (showSettingsModal = false)}
  />
{/if}
