<script lang="ts">
  import { X, Server, Globe, Box, Check, Plus, Loader2 } from 'lucide-svelte';
  import { portal } from '../utils/portal';
  import type { Instance } from '../types/instance';
  import type { RecentServer } from '../types/server';
  import { invoke } from '@tauri-apps/api/core';
  import { showNotification } from '../stores/notification';
  import { t } from '../stores/i18n';

  let { instances, onClose, onAdded } = $props<{
    instances: Instance[];
    onClose: () => void;
    onAdded: (server: RecentServer) => void;
  }>();

  let serverName = $state('');
  let serverAddress = $state('');
  let selectedInstanceId = $state(instances.length > 0 ? instances[0].id : '');
  let saving = $state(false);
  let error = $state<string | null>(null);

  async function handleSave() {
    if (!serverAddress.trim()) {
      error = $t('addServer.errAddress');
      return;
    }
    if (!selectedInstanceId) {
      error = $t('addServer.errInstance');
      return;
    }

    const inst = instances.find((i: Instance) => i.id === selectedInstanceId);
    if (!inst) {
      error = $t('addServer.errNotFound');
      return;
    }

    saving = true;
    error = null;
    try {
      const cleanAddr = serverAddress.trim();
      const finalName = serverName.trim() || cleanAddr;
      const newServer: RecentServer = {
        id: `${inst.id}_${cleanAddr}`,
        server_address: cleanAddr,
        server_name: finalName,
        instance_id: inst.id,
        instance_name: inst.name,
        instance_loader: (inst.loader || 'vanilla').toLowerCase(),
        instance_version: inst.mcVersion,
        instance_icon: inst.icon,
        icon: undefined,
        last_played: new Date().toISOString()
      };

      await invoke('add_recent_server', { server: newServer });
      showNotification($t('addServer.success', { name: finalName }), 'success');
      onAdded(newServer);
      onClose();
    } catch (err) {
      console.error('Failed to add server:', err);
      error = String(err);
    } finally {
      saving = false;
    }
  }
</script>

<div class="modal-backdrop" use:portal onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="add-server-modal-container" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <div class="modal-title-wrap">
        <div class="modal-icon-badge">
          <Server size={20} />
        </div>
        <div>
          <h2>{$t('addServer.title')}</h2>
          <p>{$t('addServer.subtitle')}</p>
        </div>
      </div>
      <button type="button" class="close-btn" onclick={onClose}>
        <X size={18} />
      </button>
    </div>

    <div class="modal-body">
      {#if error}
        <div class="modal-error-banner">
          <span>{error}</span>
        </div>
      {/if}

      <div class="form-group">
        <label for="srv-addr">{$t('addServer.address')}</label>
        <div class="input-wrap">
          <Globe size={16} class="input-icon" />
          <input
            id="srv-addr"
            type="text"
            placeholder={$t('addServer.addressPlaceholder')}
            bind:value={serverAddress}
            onkeydown={(e) => e.key === 'Enter' && handleSave()}
          />
        </div>
      </div>

      <div class="form-group">
        <label for="srv-name">{$t('addServer.name')}</label>
        <div class="input-wrap">
          <Server size={16} class="input-icon" />
          <input
            id="srv-name"
            type="text"
            placeholder={$t('addServer.namePlaceholder')}
            bind:value={serverName}
            onkeydown={(e) => e.key === 'Enter' && handleSave()}
          />
        </div>
      </div>

      <div class="form-group">
        <label for="inst-select">{$t('addServer.instance')}</label>
        <div class="input-wrap">
          <Box size={16} class="input-icon" />
          <select id="inst-select" bind:value={selectedInstanceId}>
            {#each instances as inst}
              <option value={inst.id}>
                {inst.name} ({inst.loader.toUpperCase()} {inst.mcVersion})
              </option>
            {/each}
          </select>
        </div>
        <span class="field-hint">{$t('addServer.hint')}</span>
      </div>
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-cancel" onclick={onClose} disabled={saving}>
        {$t('settings.cancel')}
      </button>
      <button type="button" class="btn-primary-green" onclick={handleSave} disabled={saving}>
        {#if saving}
          <Loader2 size={16} class="spin" />
          <span>{$t('settings.saving')}</span>
        {:else}
          <Plus size={16} />
          <span>{$t('addServer.add')}</span>
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    animation: fadeIn 0.2s ease-out;
  }

  .add-server-modal-container {
    width: 90%;
    max-width: 480px;
    background: #161a1f;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 20px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    animation: scaleUp 0.2s ease-out;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .modal-title-wrap {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .modal-icon-badge {
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: rgba(34, 197, 94, 0.12);
    border: 1px solid rgba(34, 197, 94, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #22c55e;
  }

  .modal-title-wrap h2 {
    font-size: 17px;
    font-weight: 700;
    color: #fff;
    margin: 0;
  }

  .modal-title-wrap p {
    font-size: 12px;
    color: #9ca3af;
    margin: 2px 0 0 0;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    padding: 6px;
    border-radius: 8px;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }

  .modal-body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .modal-error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
    padding: 10px 14px;
    border-radius: 10px;
    font-size: 12.5px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 12.5px;
    font-weight: 600;
    color: #e5e7eb;
  }

  .field-hint {
    font-size: 11px;
    color: #9ca3af;
    margin-top: 2px;
  }

  .input-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  :global(.input-icon) {
    position: absolute;
    left: 14px;
    color: #9ca3af;
    pointer-events: none;
  }

  .input-wrap input,
  .input-wrap select {
    width: 100%;
    padding: 11px 14px 11px 40px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    color: #fff;
    font-size: 13.5px;
    font-family: inherit;
    transition: border-color 0.2s;
    box-sizing: border-box;
  }

  .input-wrap input:focus,
  .input-wrap select:focus {
    outline: none;
    border-color: #22c55e;
    box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.2);
  }

  .input-wrap select {
    cursor: pointer;
    appearance: none;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .btn-cancel {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #d1d5db;
    padding: 9px 16px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .btn-primary-green {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: linear-gradient(135deg, #22c55e 0%, #16a34a 100%);
    border: none;
    color: #fff;
    padding: 9px 20px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 14px rgba(34, 197, 94, 0.3);
  }

  .btn-primary-green:hover:not(:disabled) {
    background: linear-gradient(135deg, #16a34a 0%, #15803d 100%);
    transform: translateY(-1px);
  }

  .btn-primary-green:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes scaleUp {
    from { transform: scale(0.96); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
