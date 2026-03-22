<template>
  <div class="bg-gray-800 rounded-lg p-4">
    <h2 class="text-xl font-bold mb-4 text-green-400">Effect Chain</h2>
    
    <div v-if="plugins.length === 0" class="text-gray-400 text-center py-8">
      No active plugins. Load one from the Plugin Manager.
    </div>
    
    <div v-else class="space-y-2">
      <div 
        v-for="(plugin, index) in plugins" 
        :key="plugin.id"
        :class="['flex items-center justify-between p-3 rounded', plugin.bypass ? 'bg-gray-600 opacity-60' : 'bg-gray-700']"
      >
        <div class="flex items-center space-x-3">
          <div class="flex flex-col space-y-1">
            <button 
              @click="movePlugin(plugin.id, -1)"
              :disabled="index === 0"
              :class="['text-xs px-2 py-0.5 rounded', index === 0 ? 'text-gray-500 cursor-not-allowed' : 'text-gray-400 hover:text-white']"
            >↑</button>
            <button 
              @click="movePlugin(plugin.id, 1)"
              :disabled="index === plugins.length - 1"
              :class="['text-xs px-2 py-0.5 rounded', index === plugins.length - 1 ? 'text-gray-500 cursor-not-allowed' : 'text-gray-400 hover:text-white']"
            >↓</button>
          </div>
          <span class="text-gray-500 font-mono">{{ index + 1 }}</span>
          <div>
            <div :class="['font-semibold', plugin.bypass ? 'line-through text-gray-400' : '']">{{ plugin.name }}</div>
            <div class="text-xs text-gray-400">{{ plugin.plugin_type }} | {{ plugin.id.slice(0, 8) }}</div>
          </div>
        </div>
        <div class="flex space-x-2">
          <button 
            @click="toggleBypass(plugin.id)"
            :class="['px-3 py-1 rounded text-sm', plugin.bypass ? 'bg-orange-500 hover:bg-orange-600' : 'bg-gray-500 hover:bg-gray-600']"
          >
            {{ plugin.bypass ? 'Enable' : 'Bypass' }}
          </button>
          <button 
            @click="selectPlugin(plugin)"
            class="px-3 py-1 bg-yellow-500 hover:bg-yellow-600 rounded text-sm"
          >
            Params
          </button>
          <button 
            @click="removePlugin(plugin.id)"
            class="px-3 py-1 bg-red-500 hover:bg-red-600 rounded text-sm"
          >
            Remove
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const plugins = computed(() => store.plugins)

async function removePlugin(id: string) {
  await store.removePlugin(id)
}

async function selectPlugin(plugin: any) {
  await store.selectPlugin(plugin)
}

async function toggleBypass(id: string) {
  try {
    await store.togglePluginBypass(id)
  } catch (e) {
    console.error('togglePluginBypass error:', e)
  }
}

async function movePlugin(id: string, direction: number) {
  try {
    await store.movePlugin(id, direction)
  } catch (e) {
    console.error('movePlugin error:', e)
  }
}
</script>
