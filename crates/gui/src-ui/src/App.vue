<template>
  <div class="min-h-screen p-4">
    <header class="mb-6">
      <h1 class="text-3xl font-bold text-blue-400">Estima</h1>
      <p class="text-gray-400">AI-Controlled Real-Time Audio Effects</p>
    </header>
    
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <PluginManager />
      <EffectChain />
      <ParameterPanel />
      <PresetManager />
    </div>
    
    <AIChat class="mt-4" />
    <BypassToggle />
  </div>
</template>

<script setup lang="ts">
import PluginManager from './components/PluginManager.vue'
import EffectChain from './components/EffectChain.vue'
import ParameterPanel from './components/ParameterPanel.vue'
import PresetManager from './components/PresetManager.vue'
import AIChat from './components/AIChat.vue'
import BypassToggle from './components/BypassToggle.vue'
</script>
