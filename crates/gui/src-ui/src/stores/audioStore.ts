import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface PluginInfo {
  id: string
  uri: string
  name: string
  plugin_type: string
  bypass: boolean
  hasUi: boolean
}

interface ParameterInfo {
  name: string
  symbol: string
  default: number
  min: number
  max: number
  current: number
  portIndex: number
}

interface ChainStatus {
  plugins: PluginInfo[]
  bypass: boolean
  lastLoadedId: string | null
}

interface CommandList {
  commands: Command[]
}

interface ParameterChangeEvent {
  pluginId: string
  portIndex: number
  value: number
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
    lastLoadedId.value = status.lastLoadedId
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

  // Listen for parameter changes from plugin UI
  async function setupParameterListener() {
    console.log('[audioStore] Setting up parameter listener...')
    const unlisten = await listen<ParameterChangeEvent>('plugin-parameter-changed', (event) => {
      const { pluginId, portIndex, value } = event.payload
      console.log('[audioStore] Received parameter change:', { pluginId, portIndex, value })
      
      // Update local parameters if this is the selected plugin
      if (selectedPlugin.value && selectedPlugin.value.id === pluginId) {
        // Create new array to force reactivity
        const idx = parameters.value.findIndex(p => p.portIndex === portIndex)
        if (idx !== -1) {
          const newParams = [...parameters.value]
          newParams[idx] = { ...newParams[idx], current: value }
          parameters.value = newParams
          console.log('[audioStore] Updated parameter:', newParams[idx].name, 'to', value)
        } else {
          console.log('[audioStore] No parameter found with portIndex:', portIndex, 'available:', parameters.value.map(p => ({ name: p.name, portIndex: p.portIndex })))
        }
      } else {
        console.log('[audioStore] Plugin not selected:', selectedPlugin.value?.id, 'vs', pluginId)
      }
    })
    console.log('[audioStore] Parameter listener registered, unlisten:', typeof unlisten)
    return unlisten
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
    setupParameterListener,
  }
})
