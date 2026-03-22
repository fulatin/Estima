<template>
  <div class="bg-gray-800 rounded-lg p-4">
    <h2 class="text-xl font-bold mb-4 text-orange-400">Presets</h2>
    
    <div class="flex space-x-2 mb-4">
      <input 
        v-model="presetName"
        type="text" 
        placeholder="Preset name..."
        class="flex-1 px-3 py-2 bg-gray-700 rounded text-white placeholder-gray-400"
      >
      <button 
        @click="savePreset"
        class="px-4 py-2 bg-green-500 hover:bg-green-600 rounded"
      >
        Save
      </button>
    </div>
    
    <div class="space-y-2 max-h-64 overflow-y-auto">
      <div 
        v-for="preset in presets" 
        :key="preset"
        class="flex items-center justify-between p-3 bg-gray-700 rounded hover:bg-gray-600"
      >
        <span>{{ preset }}</span>
        <button 
          @click="loadPreset(preset)"
          class="px-3 py-1 bg-blue-500 hover:bg-blue-600 rounded text-sm"
        >
          Load
        </button>
      </div>
    </div>
    
    <button 
      @click="refreshPresets"
      class="mt-4 w-full py-2 bg-gray-600 hover:bg-gray-500 rounded"
    >
      Refresh List
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const presetName = ref('')
const presets = ref<string[]>([])

onMounted(() => {
  refreshPresets()
})

async function refreshPresets() {
  presets.value = await store.listPresets()
}

async function savePreset() {
  if (!presetName.value) return
  await store.savePreset(presetName.value)
  presetName.value = ''
  await refreshPresets()
}

async function loadPreset(name: string) {
  await store.loadPreset(name)
}
</script>