<template>
  <div>
    <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider mb-3">
      Parameters
    </h2>
    
    <div v-if="!selectedPlugin" class="text-[#666] text-sm py-8 text-center border border-dashed border-[#333]">
      Select a plugin to edit
    </div>
    
    <div v-else class="space-y-3">
      <div class="flex items-center justify-between border-b border-[#333] pb-2 mb-3">
        <span class="text-sm font-medium text-white">{{ selectedPlugin.name }}</span>
        <span class="text-xs text-[#666] font-mono">{{ selectedPlugin.id.slice(0, 8) }}</span>
      </div>
      
      <div v-for="param in parameters" :key="param.symbol" class="space-y-1">
        <div class="flex justify-between text-xs">
          <label class="text-[#a0a0a0]">{{ param.name }}</label>
          <span class="text-[#666] font-mono">{{ param.current.toFixed(3) }}</span>
        </div>
        <input 
          type="range"
          :min="param.min"
          :max="param.max"
          :step="(param.max - param.min) / 200"
          :value="param.current"
          @input="updateParameter(param.symbol, param.name, ($event.target as HTMLInputElement).value)"
          class="w-full h-1 bg-[#333] appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:bg-amber-500 hover:[&::-webkit-slider-thumb]:bg-amber-400"
        >
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
    await store.selectPlugin(selectedPlugin.value)
  }
}
</script>
