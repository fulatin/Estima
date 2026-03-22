<template>
  <div class="bg-gray-800 rounded-lg p-4">
    <h2 class="text-xl font-bold mb-4 text-blue-400">Plugin Manager</h2>
    
    <div class="mb-4">
      <input 
        v-model="filter"
        @input="searchPlugins"
        type="text" 
        placeholder="Search plugins..."
        class="w-full px-3 py-2 bg-gray-700 rounded text-white placeholder-gray-400"
      >
    </div>
    
    <div class="max-h-96 overflow-y-auto space-y-2">
      <div 
        v-for="plugin in availablePlugins" 
        :key="plugin.id"
        class="flex items-center justify-between p-3 bg-gray-700 rounded hover:bg-gray-600"
      >
        <div>
          <div class="font-semibold">{{ plugin.name }}</div>
          <div class="text-sm text-gray-400">{{ plugin.plugin_type }}</div>
        </div>
        <button 
          @click="loadPlugin(plugin.uri)"
          class="px-3 py-1 bg-blue-500 hover:bg-blue-600 rounded text-sm"
        >
          Load
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const filter = ref('')
const availablePlugins = ref([])

onMounted(async () => {
  availablePlugins.value = await store.listPlugins()
})

async function searchPlugins() {
  availablePlugins.value = await store.listPlugins(filter.value || undefined)
}

async function loadPlugin(uri: string) {
  await store.loadPlugin(uri)
}
</script>
