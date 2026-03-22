<template>
  <div class="bg-gray-800 rounded-lg p-4">
    <h2 class="text-xl font-bold mb-4 text-purple-400">Parameters</h2>
    
    <div v-if="!selectedPlugin" class="text-gray-400 text-center py-8">
      Select a plugin from the Effect Chain to edit parameters.
    </div>
    
    <div v-else>
      <h3 class="font-semibold mb-4">{{ selectedPlugin.name }}</h3>
      
      <div class="space-y-4">
        <div v-for="param in parameters" :key="param.symbol" class="space-y-1">
          <div class="flex justify-between text-sm">
            <label>{{ param.name }}</label>
            <span class="text-gray-400">{{ param.current.toFixed(2) }}</span>
          </div>
          <input 
            type="range"
            :min="param.min"
            :max="param.max"
            :step="(param.max - param.min) / 100"
            :value="param.current"
            @input="updateParameter(param.symbol, param.name, $event.target.value)"
            class="w-full h-2 bg-gray-600 rounded-lg appearance-none cursor-pointer"
          >
          <div class="flex justify-between text-xs text-gray-500">
            <span>{{ param.min }}</span>
            <span>{{ param.max }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAudioStore } from '../stores/audioStore'

const store = useAudioStore()
const selectedPlugin = computed(() => store.selectedPlugin)
const parameters = computed(() => store.parameters)

async function updateParameter(symbol: string, name: string, value: string) {
  const numValue = parseFloat(value)
  
  if (selectedPlugin.value) {
    await store.setParameter(
      selectedPlugin.value.id,
      symbol,
      numValue
    )
    // Refresh to get updated current values
    await store.selectPlugin(selectedPlugin.value)
  }
}
</script>

<style scoped>
input[type="range"]::-webkit-slider-thumb {
  @apply w-4 h-4 bg-blue-500 rounded-full appearance-none;
}
</style>
