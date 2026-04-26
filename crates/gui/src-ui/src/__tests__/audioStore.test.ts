import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
import { useAudioStore } from '../stores/audioStore'

describe('useAudioStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('initializes with default state', () => {
    const store = useAudioStore()
    expect(store.plugins).toEqual([])
    expect(store.bypass).toBe(false)
    expect(store.lastLoadedId).toBeNull()
    expect(store.selectedPlugin).toBeNull()
    expect(store.parameters).toEqual([])
  })

  it('refreshStatus updates plugins, bypass, and lastLoadedId', async () => {
    const mockStatus = {
      plugins: [{ id: '1', uri: 'uri1', name: 'Reverb', plugin_type: 'reverb', bypass: false }],
      bypass: true,
      last_loaded_id: 'abc',
    }
    vi.mocked(invoke).mockResolvedValueOnce(mockStatus)

    const store = useAudioStore()
    await store.refreshStatus()

    expect(invoke).toHaveBeenCalledWith('get_chain_status')
    expect(store.plugins).toEqual(mockStatus.plugins)
    expect(store.bypass).toBe(true)
    expect(store.lastLoadedId).toBe('abc')
  })

  it('listPlugins calls invoke with filter', async () => {
    const mockResult = [{ id: '1', uri: 'uri1', name: 'Reverb', plugin_type: 'reverb', bypass: false }]
    vi.mocked(invoke).mockResolvedValueOnce(mockResult)

    const store = useAudioStore()
    const result = await store.listPlugins('reverb')

    expect(invoke).toHaveBeenCalledWith('list_plugins', { filter: 'reverb' })
    expect(result).toEqual(mockResult)
  })

  it('loadPlugin invokes load_plugin and refreshes', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)
    vi.mocked(invoke).mockResolvedValueOnce({
      plugins: [],
      bypass: false,
      last_loaded_id: 'new-id',
    })

    const store = useAudioStore()
    await store.loadPlugin('http://example.org/reverb')

    expect(invoke).toHaveBeenCalledWith('load_plugin', { uri: 'http://example.org/reverb' })
    expect(invoke).toHaveBeenCalledWith('get_chain_status')
  })

  it('removePlugin invokes remove_plugin and refreshes', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)
    vi.mocked(invoke).mockResolvedValueOnce({ plugins: [], bypass: false, last_loaded_id: null })

    const store = useAudioStore()
    await store.removePlugin('plugin-1')

    expect(invoke).toHaveBeenCalledWith('remove_plugin', { id: 'plugin-1' })
  })

  it('togglePluginBypass returns new state', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true)
    vi.mocked(invoke).mockResolvedValueOnce({ plugins: [], bypass: false, last_loaded_id: null })

    const store = useAudioStore()
    const result = await store.togglePluginBypass('p1')

    expect(result).toBe(true)
    expect(invoke).toHaveBeenCalledWith('toggle_plugin_bypass', { id: 'p1' })
  })

  it('movePlugin invokes move_plugin with direction', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)
    vi.mocked(invoke).mockResolvedValueOnce({ plugins: [], bypass: false, last_loaded_id: null })

    const store = useAudioStore()
    await store.movePlugin('p1', 1)

    expect(invoke).toHaveBeenCalledWith('move_plugin', { id: 'p1', direction: 1 })
  })

  it('getPluginParameters calls invoke', async () => {
    const mockParams = [{ name: 'mix', symbol: 'mix', default: 0.5, min: 0, max: 1, current: 0.5 }]
    vi.mocked(invoke).mockResolvedValueOnce(mockParams)

    const store = useAudioStore()
    const result = await store.getPluginParameters('http://example.org/reverb')

    expect(invoke).toHaveBeenCalledWith('get_plugin_parameters', { uri: 'http://example.org/reverb' })
    expect(result).toEqual(mockParams)
  })

  it('getActivePluginParameters calls invoke', async () => {
    const mockParams = [{ name: 'mix', symbol: 'mix', default: 0.5, min: 0, max: 1, current: 0.7 }]
    vi.mocked(invoke).mockResolvedValueOnce(mockParams)

    const store = useAudioStore()
    const result = await store.getActivePluginParameters('plugin-1')

    expect(invoke).toHaveBeenCalledWith('get_active_plugin_parameters', { pluginId: 'plugin-1' })
    expect(result).toEqual(mockParams)
  })

  it('selectPlugin sets selectedPlugin and loads parameters', async () => {
    const plugin = { id: 'p1', uri: 'uri1', name: 'Reverb', plugin_type: 'reverb', bypass: false }
    const mockParams = [{ name: 'mix', symbol: 'mix', default: 0.5, min: 0, max: 1, current: 0.5 }]
    vi.mocked(invoke).mockResolvedValueOnce(mockParams)

    const store = useAudioStore()
    await store.selectPlugin(plugin)

    expect(store.selectedPlugin).toEqual(plugin)
    expect(store.parameters).toEqual(mockParams)
  })

  it('setParameter calls invoke with correct args', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    const store = useAudioStore()
    await store.setParameter('p1', 'mix', 0.8)

    expect(invoke).toHaveBeenCalledWith('set_parameter', {
      pluginId: 'p1',
      paramName: 'mix',
      value: 0.8,
    })
  })

  it('toggleBypass updates bypass state', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true)

    const store = useAudioStore()
    const result = await store.toggleBypass()

    expect(result).toBe(true)
    expect(store.bypass).toBe(true)
    expect(invoke).toHaveBeenCalledWith('toggle_bypass')
  })

  it('savePreset calls invoke and returns path', async () => {
    vi.mocked(invoke).mockResolvedValueOnce('/path/to/preset.estima.json')

    const store = useAudioStore()
    const result = await store.savePreset('my-preset')

    expect(invoke).toHaveBeenCalledWith('save_preset', { name: 'my-preset' })
    expect(result).toBe('/path/to/preset.estima.json')
  })

  it('loadPreset calls invoke and refreshes', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)
    vi.mocked(invoke).mockResolvedValueOnce({ plugins: [], bypass: false, last_loaded_id: null })

    const store = useAudioStore()
    await store.loadPreset('my-preset')

    expect(invoke).toHaveBeenCalledWith('load_preset', { name: 'my-preset' })
    expect(invoke).toHaveBeenCalledWith('get_chain_status')
  })

  it('listPresets calls invoke', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(['preset1', 'preset2'])

    const store = useAudioStore()
    const result = await store.listPresets()

    expect(invoke).toHaveBeenCalledWith('list_presets')
    expect(result).toEqual(['preset1', 'preset2'])
  })

  it('aiChat calls invoke and returns CommandList', async () => {
    const mockCommands = {
      commands: [{ LoadPlugin: { uri: 'http://example.org/reverb', position: null } }],
    }
    vi.mocked(invoke).mockResolvedValueOnce(mockCommands)

    const store = useAudioStore()
    const result = await store.aiChat('add some reverb')

    expect(invoke).toHaveBeenCalledWith('ai_chat', { message: 'add some reverb' })
    expect(result).toEqual(mockCommands)
  })
})
