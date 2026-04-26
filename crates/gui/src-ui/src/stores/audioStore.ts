import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface PluginInfo {
  id: string
  uri: string
  name: string
  plugin_type: string
  bypass: boolean
}

interface ParameterInfo {
  name: string
  symbol: string
  default: number
  min: number
  max: number
  current: number
}

interface ChainStatus {
  plugins: PluginInfo[]
  bypass: boolean
  last_loaded_id: string | null
}

interface CommandList {
  commands: Command[]
}

type Command = 
  | { LoadPlugin: { uri: string; position: null } }
  | { RemovePlugin: { id: string } }
  | { SetParameter: { plugin_id: string; param_name: string; value: number } }
  | { SetBypass: { bypass: boolean } }
  | { ClearChain: {} }
  | { ShowStatus: {} }
  | { ListPlugins: { filter: string | null } }

export const useAudioStore = defineStore('audio', () => {
  const plugins = ref<PluginInfo[]>([])
  const bypass = ref(false)
  const lastLoadedId = ref<string | null>(null)
  const selectedPlugin = ref<PluginInfo | null>(null)
  const parameters = ref<ParameterInfo[]>([])

  async function refreshStatus() {
    const status: ChainStatus = await invoke('get_chain_status')
    plugins.value = status.plugins
    bypass.value = status.bypass
    lastLoadedId.value = status.last_loaded_id
  }

  async function listPlugins(filter?: string) {
    return await invoke<PluginInfo[]>('list_plugins', { filter })
  }

  async function loadPlugin(uri: string) {
    await invoke('load_plugin', { uri })
    await refreshStatus()
  }

  async function removePlugin(id: string) {
    await invoke('remove_plugin', { id })
    await refreshStatus()
  }

  async function togglePluginBypass(id: string) {
    const newState = await invoke<boolean>('toggle_plugin_bypass', { id })
    await refreshStatus()
    return newState
  }

  async function movePlugin(id: string, direction: number) {
    await invoke('move_plugin', { id, direction })
    await refreshStatus()
  }

  async function getPluginParameters(uri: string) {
    return await invoke<ParameterInfo[]>('get_plugin_parameters', { uri })
  }

  async function getActivePluginParameters(pluginId: string) {
    return await invoke<ParameterInfo[]>('get_active_plugin_parameters', { pluginId })
  }

  async function selectPlugin(plugin: PluginInfo) {
    selectedPlugin.value = plugin
    parameters.value = await getActivePluginParameters(plugin.id)
  }

  async function setParameter(pluginId: string, paramName: string, value: number) {
    await invoke('set_parameter', { 
      pluginId, 
      paramName, 
      value 
    })
  }

  async function toggleBypass() {
    const newState = await invoke<boolean>('toggle_bypass')
    bypass.value = newState
    return newState
  }

  async function savePreset(name: string) {
    return await invoke<string>('save_preset', { name })
  }

  async function loadPreset(name: string) {
    await invoke('load_preset', { name })
    await refreshStatus()
  }

  async function listPresets() {
    return await invoke<string[]>('list_presets')
  }

  async function aiChat(message: string) {
    return await invoke<CommandList>('ai_chat', { message })
  }

  async function clearHistory() {
    await invoke('clear_history')
  }

  return {
    plugins,
    bypass,
    lastLoadedId,
    selectedPlugin,
    parameters,
    refreshStatus,
    listPlugins,
    loadPlugin,
    removePlugin,
    togglePluginBypass,
    movePlugin,
    getPluginParameters,
    getActivePluginParameters,
    selectPlugin,
    setParameter,
    toggleBypass,
    savePreset,
    loadPreset,
    listPresets,
    aiChat,
    clearHistory,
  }
})
