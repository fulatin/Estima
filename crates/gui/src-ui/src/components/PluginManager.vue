<template>
  <div>
    <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider mb-3">
      Plugin Manager
    </h2>
    
    <div class="mb-3">
      <input 
        v-model="filter"
        @input="searchPlugins"
        type="text" 
        placeholder="Search plugins..."
        class="w-full px-3 py-2 bg-[#121212] border border-[#333] text-white placeholder-[#666] focus:border-amber-500 focus:outline-none text-sm"
      >
    </div>
    
    <div class="max-h-64 overflow-y-auto space-y-px">
      <div 
        v-for="plugin in availablePlugins" 
        :key="plugin.uri"
        class="flex items-center justify-between p-3 bg-[#121212] border-l-2 border-transparent hover:border-amber-500 transition-colors"
      >
        <div>
          <div class="font-medium text-sm text-white">{{ plugin.name }}</div>
          <div class="text-xs text-[#666]">{{ plugin.plugin_type }}</div>
        </div>
        <button 
          @click="loadPlugin(plugin.uri)"
          class="px-3 py-1 text-xs border border-amber-500 text-amber-400 hover:bg-amber-500/10 transition-colors"
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
const availablePlugins = ref<any[]>([])

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
